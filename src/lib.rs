//! The CS2KZ API.

use std::future::{self, Future};
use std::io;
use std::net::SocketAddr;

use anyhow::Context;
use axum::{routing, Router};
use derive_more::Debug;
use sqlx::MySql;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{error, info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use self::openapi::Security;

mod state;

#[doc(inline)]
pub use state::State;

mod config;

#[doc(inline)]
pub use config::Config;

mod http;
mod bitflags;
mod openapi;
mod steam;
mod database;
mod serde;
mod time;
mod authentication;
mod authorization;

pub mod players;
pub mod maps;
pub mod servers;
pub mod jumpstats;
pub mod records;
pub mod bans;
pub mod admins;
pub mod plugin;

/// The API.
#[derive(Debug, OpenApi)]
#[openapi(
  info(
    title = "CS2KZ API",
    description = "Source Code available on [GitHub](https://github.com/KZGlobalTeam/cs2kz-api).",
    license(
      name = "Licensed under the GPLv3",
      url = "https://www.gnu.org/licenses/gpl-3.0",
    ),
  ),
  modifiers(&Security),
  paths(),
  components(schemas()),
)]
pub struct API {
	/// The API's global state.
	state: State,

	/// A database connection pool.
	#[debug(skip)]
	database: sqlx::Pool<MySql>,

	/// A TCP listener for establishing HTTP connections.
	#[debug(skip)]
	tcp_listener: TcpListener,
}

impl API {
	/// Create a new instance of the [API].
	///
	/// To actually do anything, you need to call [`API::run()`].
	pub async fn new(
		config: Config,
		database: sqlx::Pool<MySql>,
		tcp_listener: TcpListener,
	) -> anyhow::Result<Self> {
		let state = State::new(config, database.clone()).context("initialize global state")?;

		Ok(Self {
			state,
			database,
			tcp_listener,
		})
	}

	/// Returns a reference to the database pool.
	pub const fn database(&self) -> &sqlx::Pool<MySql> {
		&self.database
	}

	/// Returns the local IP address the TCP socket is listening on.
	pub fn addr(&self) -> io::Result<SocketAddr> {
		self.tcp_listener.local_addr()
	}

	/// Generates the OpenAPI spec as JSON.
	///
	/// # Panics
	///
	/// This function will panic if the spec is invalid.
	pub fn spec() -> String {
		Self::openapi().to_pretty_json().expect("valid spec")
	}

	/// Run the API.
	///
	/// This will create an run an [`axum`] server and should never return.
	#[tracing::instrument(err)]
	pub async fn run(self) -> anyhow::Result<()> {
		self.run_until(future::pending()).await
	}

	/// Run the API until the given `until` future completes.
	///
	/// Mainly intended for testing.
	pub async fn run_until<Until>(self, until: Until) -> anyhow::Result<()>
	where
		Until: Future<Output = ()> + Send + 'static,
	{
		info!(target: "audit_log", "starting up");

		let addr = self.addr()?;
		let state: &'static State = Box::leak(Box::new(self.state));
		let swagger_ui =
			SwaggerUi::new("/docs/swagger-ui").url("/docs/openapi.json", Self::openapi());

		let api_service = Router::new()
			.route("/", routing::get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
			.nest("/players", players::router(state))
			.nest("/maps", maps::router(state))
			.nest("/servers", servers::router(state))
			.nest("/jumpstats", jumpstats::router(state))
			.nest("/records", records::router(state))
			.nest("/bans", bans::router(state))
			.nest("/admins", admins::router(state))
			.nest("/plugin", plugin::router(state))
			.layer(http::middleware::logging::layer!())
			.merge(swagger_ui)
			.into_make_service_with_connect_info::<SocketAddr>();

		info!(target: "audit_log", %addr, prod = cfg!(feature = "production"), "listening for requests");

		axum::serve(self.tcp_listener, api_service)
			.with_graceful_shutdown(async {
				tokio::select! {
					() = sigint() => {}
					() = until => {}
				}
			})
			.await
			.context("run axum")?;

		Ok(())
	}
}

/// Future that waits for CTRL-C to be pressed.
async fn sigint() {
	match signal::ctrl_c().await {
		Ok(()) => warn!(target: "audit_log", "received SIGINT; shutting down..."),
		Err(err) => error!(target: "audit_log", "failed to receive SIGINT: {err}"),
	}
}

#[cfg(test)]
mod testing;
