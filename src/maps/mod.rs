use axum::http::Method;
use axum::routing::{get, patch, put};
use axum::Router;

use crate::auth::Role;
use crate::{cors, middleware, State};

mod queries;

pub mod models;
pub use models::{CourseUpdate, CreatedMap, FilterUpdate, KZMap, MapUpdate, NewMap};

pub mod routes;

pub fn router(state: &'static State) -> Router {
	let auth = || {
		axum::middleware::from_fn_with_state(
			state,
			middleware::auth::web::layer::<{ Role::Maps as u32 }>,
		)
	};

	Router::new()
		.route("/", get(routes::get_many))
		.route_layer(cors::get())
		.route("/", put(routes::create).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PUT))
		.route("/:map", get(routes::get_single))
		.route_layer(cors::get())
		.route("/:map", patch(routes::update).route_layer(auth()))
		.route_layer(cors::dashboard(Method::PATCH))
		.with_state(state)
}

/// Helper enum for inserting mappers into the database.
#[derive(Debug)]
enum MappersTable {
	Map(u16),
	Course(u32),
}
