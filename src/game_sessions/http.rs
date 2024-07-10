//! HTTP handlers for the `/sessions` endpoint.

use axum::extract::{Path, State};
use axum::{routing, Json, Router};

use super::{GameSession, GameSessionID, GameSessionService};
use crate::middleware::cors;
use crate::openapi::responses;
use crate::Result;

impl From<GameSessionService> for Router
{
	fn from(state: GameSessionService) -> Self
	{
		Router::new()
			.route("/:id", routing::get(get_single))
			.route_layer(cors::permissive())
			.with_state(state.clone())
	}
}

/// Fetch a specific session by its ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/sessions/{session_id}",
  tag = "Sessions",
  params(("sesion_id" = u64, Path, description = "The session's ID")),
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_single(
	State(state): State<GameSessionService>,
	Path(session_id): Path<GameSessionID>,
) -> Result<Json<GameSession>>
{
	state.fetch_session(session_id).await.map(Json)
}
