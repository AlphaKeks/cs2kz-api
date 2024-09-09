//! This module contains the HTTP handlers for the `/maps` endpoint.

use axum::extract::State;
use axum::{routing, Router};
use problem_details::AsProblemDetails;

use super::{get_map, get_maps, submit_map, update_map, MapID, MapIdentifier, MapService};
use crate::extract::{Json, Path, Query};
use crate::http::{Created, NoContent};

/// Returns a router for the `/maps` endpoint.
pub fn router(map_service: MapService) -> Router
{
	Router::new()
		.route("/", routing::get(get_maps))
		.route("/", routing::put(submit_map))
		.route("/:map", routing::get(get_map))
		.route("/:map", routing::patch(update_map))
		.with_state(map_service)
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_map(
	State(map_service): State<MapService>,
	Path(map_identifier): Path<MapIdentifier>,
) -> crate::http::Result<Json<get_map::Response>>
{
	let map = match map_identifier {
		MapIdentifier::ID(map_id) => map_service.get_map_by_id(map_id).await,
		MapIdentifier::Name(map_name) => map_service.get_map_by_name(&map_name).await,
	}
	.map_err(|error| error.as_problem_details())?;

	Ok(Json(map))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_maps(
	State(map_service): State<MapService>,
	Query(request): Query<get_maps::Request>,
) -> crate::http::Result<Json<get_maps::Response>>
{
	let maps = map_service
		.get_maps(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(maps))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn submit_map(
	State(map_service): State<MapService>,
	Json(request): Json<submit_map::Request>,
) -> crate::http::Result<Created<submit_map::Response>>
{
	let response = map_service
		.submit_map(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	let location = location!("/maps/{}", response.map_id);

	Ok(Created::new(location, response))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn update_map(
	State(map_service): State<MapService>,
	Path(map_id): Path<MapID>,
	Json(request): Json<update_map::Request>,
) -> crate::http::Result<NoContent>
{
	map_service
		.update_map(map_id, request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(NoContent)
}
