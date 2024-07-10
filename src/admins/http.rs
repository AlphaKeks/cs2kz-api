//! HTTP handlers for the `/admins` endpoint.

use axum::extract::{Path, State};
use axum::http::Method;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;
use cs2kz::SteamID;

use super::{Admin, AdminService, AdminUpdate, FetchAdminsRequest};
use crate::authorization::{self, Permissions};
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::openapi::responses::{self, NoContent, PaginationResponse};
use crate::{authentication, Result};

impl From<AdminService> for Router
{
	fn from(state: AdminService) -> Self
	{
		let auth = session_auth!(
			authorization::HasPermissions<{ Permissions::ADMIN.value() }>,
			state.clone(),
		);

		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.with_state(state.clone());

		let by_id = Router::new()
			.route("/:id", routing::get(get_single))
			.route_layer(cors::permissive())
			.route("/:id", routing::put(update).route_layer(auth()))
			.route_layer(cors::dashboard([Method::PUT]))
			.with_state(state.clone());

		root.merge(by_id)
	}
}

/// Fetch admins (players with permissions).
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/admins",
  tag = "Admins",
  params(FetchAdminsRequest),
  responses(
    responses::Ok<PaginationResponse<Admin>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<AdminService>,
	Query(request): Query<FetchAdminsRequest>,
) -> Result<Json<PaginationResponse<Admin>>>
{
	state
		.fetch_admins(request)
		.await
		.map(|(admins, total)| PaginationResponse { total, results: admins })
		.map(Json)
}

/// Fetch a specific admin by their SteamID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/admins/{steam_id}",
  tag = "Admins",
  params(SteamID),
  responses(
    responses::Ok<Admin>,
    responses::NotFound,
    responses::BadRequest,
  ),
)]
pub async fn get_single(
	State(state): State<AdminService>,
	Path(steam_id): Path<SteamID>,
) -> Result<Json<Admin>>
{
	state.fetch_admin(steam_id).await.map(Json)
}

/// Create/Update an admin.
///
/// This endpoint is idempotent!
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  path = "/admins/{steam_id}",
  tag = "Admins",
  security(("Browser Session" = ["admins"])),
  params(SteamID),
  request_body = AdminUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn update(
	session: authentication::Session<authorization::HasPermissions<{ Permissions::ADMIN.value() }>>,
	State(state): State<AdminService>,
	Path(steam_id): Path<SteamID>,
	Json(update): Json<AdminUpdate>,
) -> Result<NoContent>
{
	state
		.update_admin(steam_id, update)
		.await
		.map(|()| NoContent)
}
