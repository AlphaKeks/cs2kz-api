#![feature(debug_closure_helpers)]
#![feature(decl_macro)]
#![feature(extend_one)]
#![feature(future_join)]
#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(once_cell_try_insert)]
#![feature(panic_payload_as_str)]
#![feature(panic_update_hook)]
#![feature(unqualified_local_imports)]

#[macro_use(Debug, Display, From, Error, FromStr)]
extern crate derive_more as _;

#[macro_use(Builder)]
extern crate bon as _;

#[macro_use(pin_project)]
extern crate pin_project as _;

#[macro_use(instrument, trace, debug, info, warn, error)]
extern crate tracing as _;

#[macro_use(select)]
extern crate tokio as _;

use {
	self::{config::Config, task_manager::TaskManager},
	axum::{Router, ServiceExt, extract::FromRef, handler::Handler, response::Redirect, routing},
	axum_server::{Handle as ServerHandle, Server},
	color_eyre::{
		eyre::{self, WrapErr, eyre},
		owo_colors::OwoColorize,
	},
	cs2kz_api::{
		database,
		discord,
		email,
		points::{PointsDaemon, PointsDaemonHandle},
		server_monitor::{ServerMonitor, ServerMonitorHandle},
		steam,
	},
	futures_util::FutureExt as _,
	std::{
		env,
		error::Error,
		future,
		net::{IpAddr, SocketAddr},
		path::Path,
		sync::Arc,
	},
	tokio_util::time::FutureExt as _,
	tower::ServiceBuilder,
	ulid::Ulid,
};

mod cli;
mod config;
mod http;
mod panic_hook;
mod runtime;
mod signal;
mod task_manager;
mod telemetry;

fn main() -> eyre::Result<()>
{
	color_eyre::install()?;

	#[allow(clippy::print_stderr, reason = "tracing isn't initialized yet")]
	if dotenvy::from_filename(".example.env").is_err() {
		eprintln!("{}: no {} file found", "WARNING".yellow().bold(), "`.example.env`".white());
	}

	#[allow(clippy::print_stderr, reason = "tracing isn't initialized yet")]
	if dotenvy::dotenv_override().is_err() {
		eprintln!("{}: no {} file found", "WARNING".yellow().bold(), "`.env`".white());
	}

	match cli::args() {
		cli::Args::Serve { config_path, environment, depot_downloader_path, ip_addr, port } => {
			serve(&*config_path, environment, depot_downloader_path, ip_addr, port)
		},
		cli::Args::GenerateOpenApiSchema => generate_openapi_schema(),
	}
}

fn serve(
	config_path: &Path,
	environment: Option<self::runtime::Environment>,
	depot_downloader_path: Option<Box<Path>>,
	ip_addr: Option<IpAddr>,
	port: Option<u16>,
) -> eyre::Result<()>
{
	let execution_id = Ulid::new();
	let mut config =
		Config::load_from_file(config_path).wrap_err("failed to load configuration file")?;

	if let Some(env) = environment {
		config.runtime.environment = env;
	}

	if let Some(path) = depot_downloader_path {
		config.depot_downloader.exe_path = path;
	}

	if let Some(ip) = ip_addr {
		config.http.ip_addr = ip;
	}

	if let Some(port) = port {
		config.http.port = port;
	}

	match env::var("DATABASE_URL") {
		Ok(value) => {
			config.database.url = value.parse().wrap_err("failed to parse `DATABASE_URL`")?;
		},
		Err(env::VarError::NotPresent) => {},
		Err(env::VarError::NotUnicode(raw)) => {
			warn!(?raw, "`DATABASE_URL` is set but ignored because it is invalid");
		},
	}

	self::panic_hook::install();

	let (discord_tracing_layer_handle, _tracing_guard) =
		self::telemetry::init(&config.tracing).wrap_err("failed to initialize tracing")?;

	match self::runtime::environment::set(config.runtime.environment) {
		Ok(env) => debug!(?env),
		Err(current) => warn!(?current, "runtime environment was already set elsewhere?"),
	}

	self::runtime::build(&config.runtime)
		.wrap_err("failed to build Tokio runtime")?
		.block_on(run(execution_id, config, discord_tracing_layer_handle))
}

#[allow(clippy::print_stdout)]
fn generate_openapi_schema() -> eyre::Result<()>
{
	let schema = self::http::openapi::schema()
		.to_pretty_json()
		.wrap_err("failed to serialize OpenAPI schema to JSON")?;

	println!("{schema}");

	Ok(())
}

#[instrument(skip(config, discord_tracing_layer_handle))]
async fn run(
	execution_id: Ulid,
	config: Config,
	discord_tracing_layer_handle: tracing_subscriber::reload::Handle<
		Option<cs2kz_api::discord::TracingLayer>,
		impl tracing::Subscriber,
	>,
) -> eyre::Result<()>
{
	info!(?config, "starting up");

	let config = Arc::new(config);
	let task_manager = TaskManager::default();
	let server_handle = ServerHandle::default();

	let database = database::ConnectionPool::builder()
		.url(&config.database.url)
		.maybe_min_connections(config.database.min_connections)
		.maybe_max_connections(config.database.max_connections)
		.build()
		.await
		.wrap_err("failed to connect to database")?;

	let points_daemon = PointsDaemon::new(database.clone());
	let points_daemon_handle = points_daemon.handle();
	let points_daemon_span = tracing::info_span!(parent: None, "points_daemon");
	points_daemon_span.follows_from(tracing::Span::current());

	task_manager
		.spawn(points_daemon_span, async move |cancellation_token| {
			if let Err(err) = points_daemon.run(cancellation_token).await {
				error!(error = &err as &dyn Error, "points daemon encountered an error");
			}
		})
		.wrap_err("failed to spawn points daemon task")?;

	let email_client = config.email.as_ref().map(email::Client::new).transpose()?;

	if let Some(ref client) = email_client {
		if !client.test_connection().await? {
			warn!("email connection does not seem to be working");
		}
	}

	let server_monitor_handle = if let Some(config) = config.server_monitor {
		let server_monitor = ServerMonitor::builder(config)
			.database(database.clone())
			.points_daemon(points_daemon_handle.clone())
			.maybe_email_client(email_client)
			.build();

		let server_monitor_handle = server_monitor.handle();
		let server_monitor_span = tracing::info_span!(parent: None, "server_monitor");

		task_manager
			.spawn(server_monitor_span, async move |cancellation_token| {
				if let Err(err) = server_monitor.run(cancellation_token).await {
					error!(error = &err as &dyn Error, "server monitor encountered an error");
				}
			})
			.wrap_err("failed to spawn server monitor task")?;

		server_monitor_handle
	} else {
		warn!("server monitor is disabled due to missing config");
		ServerMonitorHandle::dangling()
	};

	if let Some(config) = config.discord.clone() {
		let discord_bot = discord::Bot::new(config, database.clone())
			.wrap_err("failed to initialize discord bot")?;

		discord_tracing_layer_handle
			.reload(discord_bot.tracing_layer())
			.wrap_err("failed to load discord tracing layer")?;

		let discord_bot_span = tracing::info_span!(parent: None, "discord_bot");

		task_manager
			.spawn(discord_bot_span, async move |cancellation_token| {
				if let Err(err) = discord_bot.run(cancellation_token).await {
					error!(error = &err as &dyn Error, "discord bot encountered an error");
				}
			})
			.wrap_err("failed to spawn discord bot task")?;
	} else {
		warn!("discord bot is disabled due to missing config");
	}

	let mut router = Router::default().route("/", routing::get("(͡ ͡° ͜ つ ͡͡°)"));

	// `/docs`
	{
		router = router
			.route("/docs/openapi.json", routing::get(self::http::handlers::openapi_json))
			.route("/docs/problems.json", routing::get(self::http::handlers::problems_json));

		// In production docs.cs2kz.org is responsible for hosting a "proper" UI
		// for OpenAPI, but SwaggerUI is convenient for local development.
		if !self::runtime::environment::get().is_production() {
			router = router
				.route("/docs/swagger-ui", routing::get(Redirect::permanent("/docs/swagger-ui/")))
				.route("/docs/swagger-ui/", routing::get(self::http::handlers::swagger_ui))
				.route("/docs/swagger-ui/{*rest}", routing::get(self::http::handlers::swagger_ui));
		}
	}

	// `/leaderboards`
	{
		router = router
			.route(
				"/leaderboards/rating",
				routing::get(self::http::handlers::get_rating_leaderboard),
			)
			.route(
				"/leaderboards/records/{leaderboard}",
				routing::get(self::http::handlers::get_records_leaderboard),
			)
			.route(
				"/leaderboards/course/{course_id}/{mode}/{leaderboard}",
				routing::get(self::http::handlers::get_course_leaderboard),
			);
	}

	// `/records`
	{
		router = router
			.route("/records", routing::get(self::http::handlers::get_records))
			.route("/records/{id}", routing::get(self::http::handlers::get_record));
	}

	// `/maps`
	{
		router = router
			.route(
				"/maps",
				routing::put(self::http::handlers::create_map).get(self::http::handlers::get_maps),
			)
			.route(
				"/maps/{id}",
				routing::get(self::http::handlers::get_map).patch(self::http::handlers::update_map),
			)
			.route("/maps/{id}/state", routing::put(self::http::handlers::update_map_state));
	}

	// `/servers`
	{
		router = router
			.route(
				"/servers",
				routing::post(self::http::handlers::create_server)
					.get(self::http::handlers::get_servers),
			)
			.route(
				"/servers/{id}",
				routing::get(self::http::handlers::get_server)
					.patch(self::http::handlers::update_server),
			)
			.route(
				"/servers/{id}/access-key",
				routing::put(self::http::handlers::reset_server_access_key)
					.delete(self::http::handlers::delete_server_access_key),
			);
	}

	// `/bans`
	{
		router = router
			.route(
				"/bans",
				routing::post(self::http::handlers::create_ban).get(self::http::handlers::get_bans),
			)
			.route(
				"/bans/{id}",
				routing::get(self::http::handlers::get_ban)
					.patch(self::http::handlers::update_ban)
					.delete(self::http::handlers::revert_ban),
			);
	}

	// `/players`
	{
		router = router
			.route("/players", routing::get(self::http::handlers::get_players))
			.route("/players/{id}", routing::get(self::http::handlers::get_player))
			.route(
				"/players/{id}/preferences",
				routing::get(self::http::handlers::get_player_preferences)
					.put(self::http::handlers::update_player_preferences),
			)
			.route(
				"/players/{id}/rating",
				routing::put({
					self::http::handlers::recalculate_player_rating
						.layer(axum::middleware::from_fn(self::http::middleware::is_localhost))
				}),
			);
	}

	// `/users`
	{
		router = router
			.route("/users", routing::get(self::http::handlers::get_users))
			.route("/users/{id}", routing::get(self::http::handlers::get_user))
			.route(
				"/users/{id}/email",
				routing::put(self::http::handlers::update_user_email)
					.delete(self::http::handlers::delete_user_email),
			)
			.route(
				"/users/{id}/permissions",
				routing::put(self::http::handlers::update_user_permissions),
			)
			.route(
				"/users/{id}/server-budget",
				routing::put(self::http::handlers::update_user_server_budget),
			);
	}

	// `/mappers`
	{
		router = router.route(
			"/mappers/{id}",
			routing::put(self::http::handlers::create_mapper)
				.delete(self::http::handlers::delete_mapper),
		);
	}

	// `/events`
	{
		router = router.route("/events", routing::any(self::http::handlers::events));
	}

	// `/plugin`
	{
		router = router.route(
			"/plugin/versions",
			routing::post(self::http::handlers::create_plugin_version)
				.get(self::http::handlers::get_plugin_versions),
		);
	}

	// `/auth`
	{
		router = router
			.route("/auth/web", routing::get(self::http::handlers::web_current_session))
			.route("/auth/web/login", routing::get(self::http::handlers::web_login))
			.route("/auth/web/logout", routing::get(self::http::handlers::web_logout))
			.route(
				"/auth/web/__steam_callback",
				routing::get(self::http::handlers::steam_auth_callback),
			)
			.route("/auth/cs2", routing::any(self::http::handlers::cs2_auth));
	}

	let router = router
		.layer(self::http::middleware::auth::layer!(database.clone(), Arc::clone(&config)))
		.with_state::<()>({
			#[derive(Clone, FromRef)]
			struct State
			{
				config: Arc<Config>,
				database: database::ConnectionPool,
				task_manager: TaskManager,
				steam_api_client: steam::api::Client,
				points_daemon: PointsDaemonHandle,
				server_monitor: ServerMonitorHandle,
			}

			State {
				config: Arc::clone(&config),
				database: database.clone(),
				task_manager: task_manager.clone(),
				steam_api_client: steam::api::Client::new(&*config.steam.api_key),
				points_daemon: points_daemon_handle,
				server_monitor: server_monitor_handle,
			}
		});

	let server = Server::bind(config.http.socket_addr()).handle(server_handle.clone());
	let service = ServiceBuilder::default()
		.layer(self::http::middleware::request_id::layers())
		.layer(self::http::middleware::trace::layer(config.tracing.include_http_headers))
		.layer(self::http::middleware::safety_net::layer(
			task_manager.clone(),
			config.http.handler_timeout,
		))
		.layer(self::http::middleware::cors::layer(config.http.cors.allowed_origins()))
		.service(router)
		.into_make_service_with_connect_info::<SocketAddr>();

	let server_task_span = tracing::info_span!(parent: None, "axum");
	server_task_span.follows_from(tracing::Span::current());

	let mut server_task = task_manager
		.spawn(server_task_span, |_| server.serve(service))
		.wrap_err("failed to spawn axum task")?;

	if let Some(addr) = server_handle.listening().await {
		info!("Listening on '{addr}'");
	} else {
		let error = eyre!("server did not start up?");

		return Err(match server_task.now_or_never() {
			None => error,
			Some(Err(err)) => error.wrap_err("server failed to start up").wrap_err(err),
			Some(Ok(Ok(()))) => error.wrap_err("server exited prematurely?"),
			Some(Ok(Err(err))) => error.wrap_err("server exited prematurely?").wrap_err(err),
		});
	}

	select! {
		() = self::signal::shutdown() => {
			info!("shutting down");

			server_handle.graceful_shutdown(Some(config.http.shutdown_timeout));

			match server_task.await {
				Ok(Ok(())) => debug!("server task exited"),
				Ok(Err(err)) => error!(error = &err as &dyn Error, "server task failed to run"),
				Err(err) => {
					if err.is_panic() {
						error!(error = &err as &dyn Error, "server task panicked");
					} else {
						error!(error = &err as &dyn Error, "server task was cancelled?");
					}
				},
			}
		},

		serve_result = &mut server_task => {
			match serve_result {
				Ok(Ok(())) => warn!("server task exited prematurely"),
				Ok(Err(err)) => error!(error = &err as &dyn Error, "server task failed to run"),
				Err(err) => {
					if err.is_panic() {
						error!(error = &err as &dyn Error, "server task panicked");
					} else {
						error!(error = &err as &dyn Error, "server task was cancelled?");
					}
				},
			}
		},
	};

	let shutdown_future = future::join![
		async move {
			debug!("cleaning up database connections");
			database.shutdown().await
		},
		async move {
			debug!("shutting down tasks");
			task_manager.shutdown().await
		},
		async move {
			debug!("shutting down python thread");
			if let Err(err) = cs2kz_api::python::shutdown().await {
				warn!(error = &err as &dyn Error, "failed to shutdown python thread");
			}
		}
	];

	if let Err(_) = shutdown_future.timeout(config.http.shutdown_timeout).await {
		warn!(timeout = ?config.http.shutdown_timeout, "failed to shutdown within timeout");
	}

	Ok(())
}
