//! HTTP handlers for the `/players` endpoint.

use axum::extract::{Path, State};
use axum::{routing, Json, Router};
use axum_extra::extract::Query;
use cs2kz::SteamID;
use serde_json::Value as JsonValue;

use super::{FetchPlayersRequest, FullPlayer, NewPlayer, PlayerService, PlayerUpdate};
use crate::authentication::Jwt;
use crate::authorization::{self, Permissions};
use crate::kz::PlayerIdentifier;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, NoContent, PaginationResponse};
use crate::{authentication, steam, Result};

impl From<PlayerService> for Router
{
	fn from(state: PlayerService) -> Self
	{
		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.route("/", routing::post(register))
			.with_state(state.clone());

		let by_identifier = Router::new()
			.route("/:player", routing::get(get_single))
			.route_layer(cors::permissive())
			.route("/:player", routing::patch(update))
			.with_state(state.clone());

		let steam = Router::new()
			.route("/:player/steam", routing::get(steam))
			.route_layer(cors::permissive())
			.with_state(state.clone());

		let preferences = Router::new()
			.route("/:player/preferences", routing::get(preferences))
			.route_layer(cors::permissive())
			.with_state(state.clone());

		root.merge(by_identifier).merge(steam).merge(preferences)
	}
}

/// Fetch players.
///
/// The objects returned from this endpoint will include an `ip_address` field
/// if and only if the requesting user is authorized to manage bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players",
  tag = "Players",
  params(FetchPlayersRequest),
  responses(
    responses::Ok<PaginationResponse<FullPlayer>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
async fn get_many(
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	State(state): State<PlayerService>,
	Query(request): Query<FetchPlayersRequest>,
) -> Result<Json<PaginationResponse<FullPlayer>>>
{
	state
		.fetch_players(request, session.is_some())
		.await
		.map(|(players, total)| PaginationResponse { total, results: players })
		.map(Json)
}

/// Fetch a specific player by their name or SteamID.
///
/// The object returned from this endpoint will include an `ip_address` field if
/// and only if the requesting user is authorized to manage bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<FullPlayer>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
async fn get_single(
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	State(state): State<PlayerService>,
	Path(player): Path<PlayerIdentifier>,
) -> Result<Json<FullPlayer>>
{
	state
		.fetch_player(player, session.is_some())
		.await
		.map(Json)
}

/// Create a new player.
///
/// This endpoint is for CS2 servers. Whenever a player joins, they make a `GET`
/// request to fetch information about that player. If that request fails, they
/// will attempt to create one with this endpoint.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/players",
  tag = "Players",
  security(("CS2 Server" = [])),
  request_body = NewPlayer,
  responses(
    responses::Created,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
async fn register(
	Jwt { payload: server, .. }: Jwt<authentication::Server>,
	State(state): State<PlayerService>,
	Json(player): Json<NewPlayer>,
) -> Result<Created>
{
	state.register_player(player).await.map(Created)
}

/// Update an existing player.
///
/// This endpoint is for CS2 servers. Whenever a player disconnects, or when the
/// map changes, they will update players using this endpoint.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  path = "/players/{steam_id}",
  tag = "Players",
  security(("CS2 Server" = [])),
  params(SteamID),
  request_body = PlayerUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
async fn update(
	Jwt { payload: server, .. }: Jwt<authentication::Server>,
	State(state): State<PlayerService>,
	Path(steam_id): Path<SteamID>,
	Json(update): Json<PlayerUpdate>,
) -> Result<NoContent>
{
	state
		.update_player(steam_id, server.id(), update)
		.await
		.map(|()| NoContent)
}

/// Fetch Steam profile information for a specific player.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}/steam",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<steam::User>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
async fn steam(
	State(state): State<PlayerService>,
	Path(player): Path<PlayerIdentifier>,
) -> Result<Json<steam::User>>
{
	state.fetch_steam_profile(player).await.map(Json)
}

/// Fetch a player's in-game preferences.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/players/{player}/preferences",
  tag = "Players",
  params(PlayerIdentifier),
  responses(
    responses::Ok<responses::Object>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
async fn preferences(
	State(state): State<PlayerService>,
	Path(player): Path<PlayerIdentifier>,
) -> Result<Json<JsonValue>>
{
	state.fetch_preferences(player).await.map(Json)
}
