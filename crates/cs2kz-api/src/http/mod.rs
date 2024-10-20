use std::sync::Arc;

use axum::extract::FromRef;
use axum::{routing, Router};

use crate::config::{CookieConfig, TracingConfig};
use crate::database;

pub mod extract;
pub mod responses;
pub mod problem_details;

mod middleware;

/// Returns the top-level router.
///
/// This is what we pass to [`axum::serve()`].
pub fn router<S>(
	pool: database::ConnectionPool,
	cookie_config: impl Into<Arc<CookieConfig>>,
	tracing_config: &TracingConfig,
) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	Router::new()
		.route("/", routing::get(|| async { "(͡ ͡° ͜ つ ͡͡°)" }))
		.nest("/users", crate::users::http::router::<S>(pool, cookie_config))
		.layer(middleware::catch_panic::layer::<axum::body::Body>())
		.layer(middleware::trace::layer(tracing_config.include_http_headers))
		.layer(middleware::request_id::propagate_layer())
		.layer(middleware::request_id::set_layer())
		.merge(crate::openapi::swagger_ui())
}
