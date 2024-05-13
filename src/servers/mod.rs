//! Everything related to CS2 servers.

use axum::http::Method;
use axum::{routing, Router};

use crate::authorization::{self, Permissions};
use crate::http::cors;
use crate::http::middleware::auth::session_auth;
use crate::State;

mod queries;
mod models;

#[doc(inline)]
pub use models::{
	CreatedServer, CreatedServerKey, NewServer, Server, ServerID, ServerUpdate, TokenRequest,
};

pub mod handlers;

/// Returns a [Router] for `/servers`.
pub fn router(state: &'static State) -> Router {
	let is_admin = session_auth!(
		state,
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>
	);

	let is_admin_or_owner = session_auth!(state, authorization::IsServerAdminOrOwner);

	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route(
			"/",
			routing::post(handlers::root::post).route_layer(is_admin()),
		)
		.route_layer(cors::dashboard(Method::POST))
		.with_state(state);

	let token = Router::new()
		.route("/token", routing::post(handlers::token::generate))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:server", routing::get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route(
			"/:server",
			routing::patch(handlers::by_identifier::patch).route_layer(is_admin_or_owner()),
		)
		.route_layer(cors::dashboard(Method::PATCH))
		.with_state(state);

	let key = Router::new()
		.route(
			"/:server/key",
			routing::put(handlers::key::replace).route_layer(is_admin_or_owner()),
		)
		.route(
			"/:server/key",
			routing::delete(handlers::key::delete).route_layer(is_admin()),
		)
		.route_layer(cors::dashboard([Method::PUT, Method::DELETE]))
		.with_state(state);

	root.merge(token).merge(by_identifier).merge(key)
}
