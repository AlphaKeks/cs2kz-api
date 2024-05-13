//! Handlers for the `/players/{player}/steam` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::PlayerIdentifier;

use crate::database::ResolveID;
use crate::http::{HandlerError, HandlerResult};
use crate::openapi::{parameters, responses};
use crate::{steam, State};

/// Fetch information about a specific player from Steam.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/{player}/steam",
  params(parameters::PlayerIdentifier),
  responses(
    responses::Ok<steam::User>,
    responses::NoContent,
    responses::BadRequest,
    responses::BadGateway,
  ),
)]
pub async fn get(
	state: &State,
	Path(player): Path<PlayerIdentifier>,
) -> HandlerResult<Json<steam::User>> {
	let steam_id = player
		.resolve_id(&state.database)
		.await?
		.ok_or_else(|| HandlerError::unknown("player"))?;

	let steam_user = steam::User::fetch(steam_id, &state.http_client, &state.config).await?;

	Ok(Json(steam_user))
}
