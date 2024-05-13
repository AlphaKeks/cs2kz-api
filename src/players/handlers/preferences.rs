//! Handlers for the `/players/{player}/preferences` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;
use serde_json::Value as JsonValue;
use sqlx::types::Json as SqlJson;
use sqlx::QueryBuilder;

use crate::http::{HandlerError, HandlerResult};
use crate::openapi::{parameters, responses};
use crate::State;

/// Fetch a player's in-game preferences.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/{player}/preferences",
  params(parameters::PlayerIdentifier),
  responses(
    responses::Ok<responses::JsonObject>,
    responses::NoContent,
    responses::BadRequest,
    responses::BadGateway,
  ),
)]
pub async fn get(
	state: &State,
	Path(player): Path<PlayerIdentifier>,
) -> HandlerResult<Json<JsonValue>> {
	let mut query = QueryBuilder::new("SELECT game_preferences FROM Players WHERE");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push(" id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push(" name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let SqlJson(preferences) = query
		.build_query_scalar::<SqlJson<JsonValue>>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| HandlerError::no_content())?;

	Ok(Json(preferences))
}
