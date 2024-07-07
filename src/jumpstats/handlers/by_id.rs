//! HTTP handlers for the `/jumpstats/{jumpstat_id}` routes.

use axum::extract::Path;
use axum::Json;
use sqlx::QueryBuilder;

use crate::jumpstats::{queries, Jumpstat, JumpstatID};
use crate::openapi::responses;
use crate::{Error, Result, State};

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
pub async fn get(state: State, Path(jumpstat_id): Path<JumpstatID>) -> Result<Json<Jumpstat>>
{
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE j.id = ").push_bind(jumpstat_id);

	let jumpstat = query
		.build_query_as::<Jumpstat>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| Error::not_found("jumpstat"))?;

	Ok(Json(jumpstat))
}
