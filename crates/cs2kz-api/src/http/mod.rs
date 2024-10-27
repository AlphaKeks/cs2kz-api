//! Everything related to HTTP.

use std::sync::Arc;

use axum::extract::FromRef;
use axum::{Router, routing};

use crate::config::Config;
use crate::database;

pub mod extract;
pub mod middleware;
pub mod problem_details;
pub mod response;

pub(crate) mod openapi;
pub(crate) mod players;
pub(crate) mod plugin;
pub(crate) mod servers;
pub(crate) mod users;

/// Returns the API's top-level [`Router`].
pub fn router<S>(database: database::ConnectionPool, config: Config) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let database = || database.clone();
	let cookie_config = Arc::new(config.http.cookies);
	let include_http_headers = config.tracing.include_http_headers;

	Router::new()
		.route("/", routing::get("(͡ ͡° ͜ つ ͡͡°)"))
		.nest("/docs", openapi::router())
		.nest(
			"/users",
			users::router(database(), Arc::clone(&cookie_config)),
		)
		.nest("/plugin", plugin::router(&config.credentials, database()))
		.nest("/players", players::router())
		.nest(
			"/servers",
			servers::router(database(), Arc::clone(&cookie_config)),
		)
		.layer(middleware::catch_panic::layer::<axum::body::Body>())
		.layer(middleware::trace::layer(include_http_headers))
		.layer(middleware::request_id::propagate_layer())
		.layer(middleware::request_id::set_layer())
}
