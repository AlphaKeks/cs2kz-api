//! HTTP handlers for the `/plugin` endpoint.

use axum::extract::State;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{
	CreatedPluginVersion,
	FetchVersionsRequest,
	NewPluginVersion,
	PluginService,
	PluginVersion,
};
use crate::authentication::ApiKey;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, PaginationResponse};
use crate::{Error, Result};

impl From<PluginService> for Router
{
	fn from(state: PluginService) -> Self
	{
		Router::new()
			.route("/versions", routing::get(get_versions))
			.route_layer(cors::permissive())
			.route("/versions", routing::post(submit_version))
			.with_state(state.clone())
	}
}

/// Fetch CS2KZ plugin versions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  params(FetchVersionsRequest),
  responses(
    responses::Ok<PaginationResponse<PluginVersion>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_versions(
	State(state): State<PluginService>,
	Query(request): Query<FetchVersionsRequest>,
) -> Result<Json<PaginationResponse<PluginVersion>>>
{
	state
		.fetch_versions(request)
		.await
		.map(|(versions, total)| PaginationResponse { total, results: versions })
		.map(Json)
}

/// Submit a new CS2KZ plugin version.
///
/// This endpoint is intended to be used by GitHub Actions.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/plugin/versions",
  tag = "CS2KZ Plugin",
  security(("API Key" = ["plugin_versions"])),
  request_body = NewPluginVersion,
  responses(
    responses::Created<CreatedPluginVersion>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn submit_version(
	api_key: ApiKey,
	State(state): State<PluginService>,
	Json(version): Json<NewPluginVersion>,
) -> Result<Created<Json<CreatedPluginVersion>>>
{
	if api_key.name() != "plugin_versions" {
		return Err(Error::unauthorized().context(api_key.to_string()));
	}

	state.submit_version(version).await.map(Json).map(Created)
}
