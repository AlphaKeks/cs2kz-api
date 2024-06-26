//! Everything related to KZ player bans.

use axum::http::Method;
use axum::{routing, Router};

use crate::authorization::Permissions;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::{authorization, State};

mod models;
pub use models::{
	Ban, BanID, BanReason, BanUpdate, CreatedBan, CreatedUnban, NewBan, NewUnban, Unban, UnbanID,
};

mod queries;
pub mod handlers;

/// Returns an [`axum::Router`] for the `/bans` routes.
pub fn router(state: State) -> Router {
	let auth = session_auth!(
		authorization::HasPermissions<{ Permissions::BANS.value() }>,
		state.clone(),
	);

	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::post(handlers::root::post))
		.route_layer(cors::dashboard([Method::POST]))
		.with_state(state.clone());

	let by_id = Router::new()
		.route("/:id", routing::get(handlers::by_id::get))
		.route_layer(cors::permissive())
		.route(
			"/:id",
			routing::patch(handlers::by_id::patch).route_layer(auth()),
		)
		.route(
			"/:id",
			routing::delete(handlers::by_id::delete).route_layer(auth()),
		)
		.route_layer(cors::dashboard([Method::PATCH, Method::DELETE]))
		.with_state(state.clone());

	root.merge(by_id)
}
