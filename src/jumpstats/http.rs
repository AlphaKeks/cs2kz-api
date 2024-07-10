//! HTTP handlers for the `/jumpstats` endpoint.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{FetchJumpstatsRequest, Jumpstat, JumpstatID, JumpstatService, NewJumpstat};
use crate::authentication::{self, Jwt};
use crate::jumpstats::CreatedJumpstat;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, PaginationResponse};
use crate::Result;

impl From<JumpstatService> for Router
{
	fn from(state: JumpstatService) -> Self
	{
		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.route("/", routing::post(submit))
			.with_state(state.clone());

		let by_id = Router::new()
			.route("/:id", routing::get(get_single))
			.route_layer(cors::permissive())
			.with_state(state.clone());

		let replay = Router::new()
			.route("/:id/replay", routing::get(get_replay))
			.route_layer(cors::permissive())
			.with_state(state.clone());

		root.merge(by_id).merge(replay)
	}
}

/// Fetch jumpstats.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/jumpstats",
  tag = "Jumpstats",
  params(FetchJumpstatsRequest),
  responses(
    responses::Ok<PaginationResponse<Jumpstat>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<JumpstatService>,
	Query(request): Query<FetchJumpstatsRequest>,
) -> Result<Json<PaginationResponse<Jumpstat>>>
{
	state
		.fetch_jumpstats(request)
		.await
		.map(|(jumpstats, total)| PaginationResponse { total, results: jumpstats })
		.map(Json)
}

/// Fetch a specific jumpstat by its ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/jumpstats/{jumpstat_id}",
  tag = "Jumpstats",
  params(("jumpstat_id" = u64, Path, description = "The jumpstat's ID")),
  responses(
    responses::Ok<Jumpstat>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_single(
	State(state): State<JumpstatService>,
	Path(jumpstat_id): Path<JumpstatID>,
) -> Result<Json<Jumpstat>>
{
	state.fetch_jumpstat(jumpstat_id).await.map(Json)
}

/// Create a new jumpstat.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/jumpstats",
  tag = "Jumpstats",
  security(("CS2 Server" = [])),
  request_body = NewJumpstat,
  responses(
    responses::Created<CreatedJumpstat>,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn submit(
	Jwt { payload: server, .. }: Jwt<authentication::Server>,
	State(state): State<JumpstatService>,
	Json(jumpstat): Json<NewJumpstat>,
) -> Result<Created<Json<CreatedJumpstat>>>
{
	state
		.submit_jumpstat(jumpstat, server)
		.await
		.map(Json)
		.map(Created)
}

/// Fetch a jumpstat replay.
#[tracing::instrument]
#[utoipa::path(
  get,
  path = "/jumpstats/{jumpstat_id}/replay",
  tag = "Jumpstats",
  params(("jumpstat_id" = u64, Path, description = "The jumpstat's ID")),
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_replay(Path(jumpstat_id): Path<JumpstatID>) -> StatusCode
{
	StatusCode::SERVICE_UNAVAILABLE
}
