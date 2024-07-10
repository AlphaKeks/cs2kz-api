//! HTTP handlers for the `/records` endpoint.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{CreatedRecord, FetchRecordsRequest, NewRecord, Record, RecordID, RecordService};
use crate::authentication::Jwt;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, PaginationResponse};
use crate::{authentication, Result};

impl From<RecordService> for Router
{
	fn from(state: RecordService) -> Self
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

/// Fetch records.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/records",
  tag = "Records",
  params(FetchRecordsRequest),
  responses(
    responses::Ok<PaginationResponse<Record>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<RecordService>,
	Query(request): Query<FetchRecordsRequest>,
) -> Result<Json<PaginationResponse<Record>>>
{
	state
		.fetch_records(request)
		.await
		.map(|(records, total)| PaginationResponse { total, results: records })
		.map(Json)
}

/// Fetch a specific record by its ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/records/{record_id}",
  tag = "Records",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<Record>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_single(
	State(state): State<RecordService>,
	Path(record_id): Path<RecordID>,
) -> Result<Json<Record>>
{
	state.fetch_record(record_id).await.map(Json)
}

/// Create a new record.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/records",
  tag = "Records",
  security(("CS2 Server" = [])),
  request_body = NewRecord,
  responses(
    responses::Created<CreatedRecord>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn submit(
	Jwt { payload: server, .. }: Jwt<authentication::Server>,
	State(state): State<RecordService>,
	Json(record): Json<NewRecord>,
) -> Result<Created<Json<CreatedRecord>>>
{
	state
		.submit_record(record, server)
		.await
		.map(Json)
		.map(Created)
}

/// Fetch a record replay.
#[tracing::instrument]
#[utoipa::path(
  get,
  path = "/records/{record_id}/replay",
  tag = "Records",
  params(("record_id" = u64, Path, description = "The record's ID")),
  responses(
    responses::Ok<()>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_replay(Path(record_id): Path<RecordID>) -> StatusCode
{
	StatusCode::SERVICE_UNAVAILABLE
}
