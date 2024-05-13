//! Module containing everything related to KZ maps.

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
	Course, CourseFilter, CourseID, CourseUpdate, CreatedMap, FilterID, FilterUpdate,
	FilterUpdates, FullMap, MapID, MapUpdate, NewCourse, NewFilter, NewMap,
};

pub mod handlers;

/// Returns a [Router] for `/maps`.
pub fn router(state: &'static State) -> Router {
	let auth = session_auth!(
		state,
		authorization::HasPermissions<{ Permissions::MAPS.value() }>
	);

	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::put(handlers::root::put).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PUT))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:map", routing::get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route(
			"/:map",
			routing::patch(handlers::by_identifier::patch).route_layer(auth()),
		)
		.route_layer(cors::dashboard(Method::PATCH))
		.with_state(state);

	root.merge(by_identifier)
}
