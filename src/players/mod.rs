//! Module containing everything related to KZ players.

use axum::{routing, Router};

use crate::http::cors;
use crate::State;

mod queries;
mod models;

#[doc(inline)]
pub use models::{
	CourseSessionData, CourseSessionID, CreatedGameSession, FullPlayer, GameSessionData,
	GameSessionID, NewPlayer, Player, PlayerUpdate,
};

pub mod handlers;

/// Returns a [Router] for `/players`.
pub fn router(state: &'static State) -> Router {
	let root = Router::new()
		.route("/", routing::get(handlers::root::get))
		.route_layer(cors::permissive())
		.route("/", routing::post(handlers::root::post))
		.with_state(state);

	let by_identifier = Router::new()
		.route("/:player", routing::get(handlers::by_identifier::get))
		.route_layer(cors::permissive())
		.route("/:player", routing::patch(handlers::by_identifier::patch))
		.with_state(state);

	let steam = Router::new()
		.route("/:player/steam", routing::get(handlers::steam::get))
		.route_layer(cors::permissive())
		.with_state(state);

	let preferences = Router::new()
		.route(
			"/:player/preferences",
			routing::get(handlers::preferences::get),
		)
		.route_layer(cors::permissive())
		.with_state(state);

	root.merge(by_identifier).merge(steam).merge(preferences)
}
