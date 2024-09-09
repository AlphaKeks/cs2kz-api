//! This module contains the HTTP handlers for the `/players` endpoint.

use axum::extract::State;
use axum::{routing, Router};
use cs2kz::SteamID;
use problem_details::AsProblemDetails;

use super::{
	get_player,
	get_players,
	get_preferences,
	PlayerIdentifier,
	PlayerService,
	Preferences,
};
use crate::extract::{Json, Path, Query};
use crate::services::steam;

/// Returns a router for the `/players` endpoint.
pub fn router(player_service: PlayerService) -> Router
{
	Router::new()
		.route("/", routing::get(get_players))
		.route("/:player", routing::get(get_player))
		.route("/:player/preferences", routing::get(get_preferences))
		.route("/:player/steam", routing::get(get_steam_profile))
		.with_state(player_service)
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_player(
	State(player_service): State<PlayerService>,
	Path(player_identifier): Path<PlayerIdentifier>,
) -> crate::http::Result<Json<get_player::Response>>
{
	let player = match player_identifier {
		PlayerIdentifier::ID(steam_id) => player_service.get_player_by_id(steam_id).await,
		PlayerIdentifier::Name(name) => player_service.get_player_by_name(&name).await,
	}
	.map_err(|error| error.as_problem_details())?;

	Ok(Json(player))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_players(
	State(player_service): State<PlayerService>,
	Query(request): Query<get_players::Request>,
) -> crate::http::Result<Json<get_players::Response>>
{
	let players = player_service
		.get_players(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(players))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_preferences(
	State(player_service): State<PlayerService>,
	Path(player_identifier): Path<PlayerIdentifier>,
) -> crate::http::Result<Json<Preferences>>
{
	let steam_id = player_identifier
		.resolve_id(&player_service.mysql)
		.await
		.map_err(get_preferences::Error::from)
		.map_err(|error| error.as_problem_details())?
		.ok_or(get_preferences::Error::PlayerNotFound)
		.map_err(|error| error.as_problem_details())?;

	let preferences = player_service
		.get_player_preferences(steam_id)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(preferences))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_steam_profile(
	State(player_service): State<PlayerService>,
	Path(steam_id): Path<SteamID>,
) -> crate::http::Result<Json<steam::User>>
{
	let user = player_service
		.steam_service
		.get_user(steam_id)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(user))
}
