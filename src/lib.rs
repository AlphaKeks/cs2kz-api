#![forbid(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)]
#![deny(
	clippy::cast_possible_truncation,
	clippy::cast_possible_wrap,
	clippy::cast_sign_loss,
	clippy::checked_conversions,
	clippy::many_single_char_names,
	clippy::missing_panics_doc,
	clippy::needless_for_each,
	clippy::ref_option_ref,
	clippy::unimplemented,
	clippy::unnecessary_self_imports,
	clippy::wildcard_dependencies,
	clippy::wildcard_imports
)]
#![warn(
	clippy::style,
	clippy::perf,
	clippy::absolute_paths,
	clippy::branches_sharing_code,
	clippy::cloned_instead_of_copied,
	clippy::cognitive_complexity,
	clippy::collection_is_never_read,
	clippy::dbg_macro,
	clippy::enum_glob_use,
	clippy::inconsistent_struct_constructor,
	clippy::mismatching_type_param_order,
	clippy::missing_const_for_fn,
	clippy::needless_continue,
	clippy::needless_pass_by_ref_mut,
	clippy::needless_pass_by_value,
	clippy::option_if_let_else,
	clippy::redundant_else,
	clippy::semicolon_if_nothing_returned,
	clippy::semicolon_outside_block,
	clippy::similar_names,
	clippy::todo,
	clippy::unnested_or_patterns,
	clippy::unused_async,
	clippy::use_self
)]

use std::io;
use std::net::SocketAddr;

use axum::Router;
use color_eyre::eyre::Context;
use itertools::Itertools;
use tokio::net::TcpListener;
use tracing::debug;
use utoipa::OpenApi;

use self::auth::openapi::Security;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use cs2kz_api_macros::test;

mod error;
pub use error::{Error, Result};

pub mod config;
pub use config::Config;

mod state;
pub use state::State;

/// Convenience alias for extracting [`State`] in handlers.
pub type AppState = axum::extract::State<&'static crate::State>;

#[doc(hidden)]
pub mod env;

mod cors;
mod database;
mod middleware;
mod params;
mod query;
mod responses;
mod serde;
mod sqlx;
mod status;
mod steam;

mod admins;
mod auth;
mod bans;
mod course_sessions;
mod docs;
mod jumpstats;
mod maps;
mod players;
mod records;
mod servers;
mod sessions;

#[derive(OpenApi)]
#[rustfmt::skip]
#[openapi(
  info(
    title = "CS2KZ API",
    license(
      name = "GPL-3.0",
      url = "https://www.gnu.org/licenses/gpl-3.0",
    ),
  ),
  modifiers(&Security),
  components(
    schemas(
      cs2kz::SteamID,
      cs2kz::Mode,
      cs2kz::Style,
      cs2kz::Jumpstat,
      cs2kz::Tier,
      cs2kz::PlayerIdentifier,
      cs2kz::MapIdentifier,
      cs2kz::ServerIdentifier,

      error::Error,

      params::Limit,
      params::Offset,

      database::RankedStatus,
      database::GlobalStatus,

      maps::models::KZMap,
      maps::models::Course,
      maps::models::Filter,
      maps::models::NewMap,
      maps::models::NewCourse,
      maps::models::NewFilter,
      maps::models::MapUpdate,
      maps::models::CourseUpdate,
      maps::models::FilterUpdate,

      servers::models::Server,
      servers::models::NewServer,
      servers::models::CreatedServer,
      servers::models::ServerUpdate,

      players::models::Player,
      players::models::FullPlayer,
      players::models::NewPlayer,
      players::models::PlayerUpdate,
      players::models::PlayerUpdateSession,
      players::models::PlayerUpdateCourseSession,

      bans::models::Ban,
      bans::models::BannedPlayer,
      bans::models::Unban,
      bans::models::NewBan,
      bans::models::CreatedBan,
      bans::models::BanUpdate,
      bans::models::NewUnban,
      bans::models::CreatedUnban,

      admins::models::Admin,

      auth::Role,
      auth::RoleFlags,

      sessions::models::Session,
      sessions::models::TimeSpent,
      sessions::models::BhopStats,

      course_sessions::models::CourseSession,
    ),
  ),
  paths(
    status::status,

    maps::routes::get_many::get_many,
    maps::routes::create::create,
    maps::routes::get_single::get_single,
    maps::routes::update::update,

    servers::routes::get_many::get_many,
    servers::routes::create::create,
    servers::routes::get_single::get_single,
    servers::routes::update::update,
    servers::routes::replace_key::replace_key,
    servers::routes::delete_key::delete_key,

    players::routes::get_many::get_many,
    players::routes::create::create,
    players::routes::get_single::get_single,
    players::routes::update::update,

    bans::routes::get_many::get_many,
    bans::routes::create::create,
    bans::routes::get_single::get_single,
    bans::routes::update::update,
    bans::routes::unban::unban,

    admins::routes::get_many::get_many,
    admins::routes::get_single::get_single,
    admins::routes::update::update,

    auth::routes::login::login,
    auth::routes::logout::logout,
    auth::steam::routes::callback::callback,

    sessions::routes::get_many::get_many,
    sessions::routes::get_single::get_single,

    course_sessions::routes::get_many::get_many,
    course_sessions::routes::get_single::get_single,
  ),
)]
pub struct API {
	/// The TCP listener used by the underlying HTTP server.
	tcp_listener: TcpListener,

	/// The global application state.
	state: State,
}

impl API {
	/// Creates a new API instance with the given `config`.
	///
	/// See [`API::run()`] for starting the server.
	#[tracing::instrument]
	pub async fn new(config: Config) -> state::Result<Self> {
		let tcp_listener = TcpListener::bind(config.socket_addr)
			.await
			.expect("failed to bind to TCP socket");

		let local_addr = tcp_listener
			.local_addr()
			.expect("failed to get TCP address");

		debug!(%local_addr, "Initialized TCP socket");

		let state = State::new(config).await?;

		debug!("Initialized API state");

		Ok(Self { tcp_listener, state })
	}

	/// Runs the [axum] server for the API.
	#[tracing::instrument(skip(self))]
	pub async fn run(self) -> color_eyre::Result<()> {
		let state: &'static _ = Box::leak(Box::new(self.state));

		let router = Router::new()
			.nest("/", status::router())
			.nest("/docs", docs::router())
			.nest("/maps", maps::router(state))
			.nest("/servers", servers::router(state))
			.nest("/records", records::router(state))
			.nest("/jumpstats", jumpstats::router(state))
			.nest("/players", players::router(state))
			.nest("/bans", bans::router(state))
			.nest("/admins", admins::router(state))
			.nest("/auth", auth::router(state))
			.nest("/sessions", sessions::router(state))
			.nest("/course-sessions", course_sessions::router(state))
			.layer(middleware::logging::layer!())
			.into_make_service();

		audit!("starting axum server", prod = %cfg!(feature = "production"));

		axum::serve(self.tcp_listener, router)
			.await
			.context("failed to run axum")
	}

	/// Returns the local socket address for the underlying TCP server.
	pub fn local_addr(&self) -> io::Result<SocketAddr> {
		self.tcp_listener.local_addr()
	}

	/// Returns an iterator over all the routes registered in the OpenAPI spec.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_iter().map(|(uri, path)| {
			let methods = path
				.operations
				.into_keys()
				.map(|method| format!("{method:?}").to_uppercase())
				.collect_vec()
				.join(", ");

			format!("`{uri}` [{methods}]")
		})
	}

	/// Returns a pretty-printed version of the OpenAPI spec in JSON.
	///
	/// # Panics
	///
	/// This will panic if any types used in the spec cannot be serialized as JSON.
	pub fn spec() -> String {
		Self::openapi()
			.to_pretty_json()
			.expect("Failed to format API spec as JSON.")
	}
}

/// Logs a message with `audit = true`.
///
/// This will cause the log to be saved in the database.
#[macro_export]
macro_rules! audit {
	($level:ident, $message:literal $(,$($fields:tt)*)?) => {
		::tracing::$level!(target: "audit_log", $($($fields)*,)? $message)
	};

	($message:literal $(,$($fields:tt)*)?) => {
		audit!(trace, $message $(,$($fields)*)?)
	};
}

#[cfg(test)]
mod test_setup {
	use tracing_subscriber::EnvFilter;

	#[ctor::ctor]
	fn test_setup() {
		color_eyre::install().unwrap();
		dotenvy::dotenv().unwrap();
		tracing_subscriber::fmt()
			.with_env_filter(EnvFilter::from_default_env())
			.init();
	}
}
