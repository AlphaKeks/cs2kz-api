//! HTTP handlers for the `/maps` endpoint.

use axum::extract::{Path, State};
use axum::http::Method;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{CreatedMap, FetchMapsRequest, FullMap, MapID, MapService, MapUpdate, NewMap};
use crate::authorization::{self, Permissions};
use crate::kz::MapIdentifier;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, NoContent, PaginationResponse};
use crate::{authentication, Result};

impl From<MapService> for Router
{
	fn from(state: MapService) -> Self
	{
		let auth = session_auth!(
			authorization::HasPermissions<{ Permissions::MAPS.value() }>,
			state.clone(),
		);

		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.route("/", routing::put(submit).route_layer(auth()))
			.route_layer(cors::dashboard([Method::PUT]))
			.with_state(state.clone());

		let by_identifier = Router::new()
			.route("/:map", routing::get(get_single))
			.route_layer(cors::permissive())
			.route("/:map", routing::patch(update).route_layer(auth()))
			.route_layer(cors::dashboard([Method::PATCH]))
			.with_state(state.clone());

		root.merge(by_identifier)
	}
}

/// Fetch maps.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/maps",
  tag = "Maps",
  params(FetchMapsRequest),
  responses(
    responses::Ok<PaginationResponse<FullMap>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<MapService>,
	Query(request): Query<FetchMapsRequest>,
) -> Result<Json<PaginationResponse<FullMap>>>
{
	state
		.fetch_maps(request)
		.await
		.map(|(maps, total)| PaginationResponse { total, results: maps })
		.map(Json)
}

/// Fetch a specific map by its name or ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/maps/{map}",
  tag = "Maps",
  params(MapIdentifier),
  responses(
    responses::Ok<FullMap>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_single(
	State(state): State<MapService>,
	Path(map): Path<MapIdentifier>,
) -> Result<Json<FullMap>>
{
	state.fetch_map(map).await.map(Json)
}

/// Create a new map.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  path = "/maps",
  tag = "Maps",
  security(("Browser Session" = ["maps"])),
  request_body = NewMap,
  responses(
    responses::Created<CreatedMap>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn submit(
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	State(state): State<MapService>,
	Json(map): Json<NewMap>,
) -> Result<Created<Json<CreatedMap>>>
{
	state.submit_map(map).await.map(Json).map(Created)
}

/// Update an existing map.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  path = "/maps/{map_id}",
  tag = "Maps",
  security(("Browser Session" = ["maps"])),
  params(("map_id" = u16, Path, description = "The map's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn update(
	session: authentication::Session<authorization::HasPermissions<{ Permissions::MAPS.value() }>>,
	State(state): State<MapService>,
	Path(map_id): Path<MapID>,
	Json(update): Json<MapUpdate>,
) -> Result<NoContent>
{
	state.update_map(map_id, update).await.map(|()| NoContent)
}
