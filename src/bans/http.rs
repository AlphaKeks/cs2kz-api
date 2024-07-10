//! HTTP handlers for the `/bans` endpoint.

use axum::extract::{Path, State};
use axum::http::Method;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{
	Ban,
	BanID,
	BanService,
	BanUpdate,
	CreatedBan,
	CreatedUnban,
	FetchBansRequest,
	NewBan,
	NewUnban,
};
use crate::authentication::{self, Jwt};
use crate::authorization::{self, Permissions};
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, NoContent, PaginationResponse};
use crate::{Error, Result};

impl From<BanService> for Router
{
	fn from(state: BanService) -> Self
	{
		let auth = session_auth!(
			authorization::HasPermissions<{ Permissions::BANS.value() }>,
			state.clone(),
		);

		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.route("/", routing::post(submit))
			.route_layer(cors::dashboard([Method::POST]))
			.with_state(state.clone());

		let by_id = Router::new()
			.route("/:id", routing::get(get_single))
			.route_layer(cors::permissive())
			.route("/:id", routing::patch(update).route_layer(auth()))
			.route("/:id", routing::delete(unban).route_layer(auth()))
			.route_layer(cors::dashboard([Method::PATCH, Method::DELETE]))
			.with_state(state.clone());

		root.merge(by_id)
	}
}

/// Fetch bans.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/bans",
  tag = "Bans",
  params(FetchBansRequest),
  responses(
    responses::Ok<PaginationResponse<Ban>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<BanService>,
	Query(request): Query<FetchBansRequest>,
) -> Result<Json<PaginationResponse<Ban>>>
{
	state
		.fetch_bans(request)
		.await
		.map(|(bans, total)| PaginationResponse { total, results: bans })
		.map(Json)
}

/// Fetch a specific ban by its ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/bans/{ban_id}",
  tag = "Bans",
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Ok<Ban>,
    responses::NotFound,
    responses::BadRequest,
  ),
)]
pub async fn get_single(
	State(state): State<BanService>,
	Path(ban_id): Path<BanID>,
) -> Result<Json<Ban>>
{
	state.fetch_ban(ban_id).await.map(Json)
}

/// Create a new ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/bans",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  request_body = NewBan,
  responses(
    responses::Created<CreatedBan>,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
#[axum::debug_handler]
pub async fn submit(
	server: Option<Jwt<authentication::Server>>,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	State(state): State<BanService>,
	Json(ban): Json<NewBan>,
) -> Result<Created<Json<CreatedBan>>>
{
	let (server, admin) = match (server, session) {
		(Some(server), None) => (Some(server.into_payload()), None),
		(None, Some(session)) => (None, Some(session.user())),
		(None, None) => {
			return Err(Error::unauthorized());
		}
		(Some(server), Some(session)) => {
			tracing::warn! {
				target: "cs2kz_api::audit_log",
				?server,
				?session,
				"request authenticated both as server and session",
			};

			return Err(Error::unauthorized());
		}
	};

	state
		.submit_ban(ban, server, admin)
		.await
		.map(Json)
		.map(Created)
}

/// Update an existing ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn update(
	session: authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	State(state): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(update): Json<BanUpdate>,
) -> Result<NoContent>
{
	if update.is_empty() {
		return Ok(NoContent);
	}

	state.update_ban(ban_id, update).await.map(|()| NoContent)
}

/// Revert a ban.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  path = "/bans/{ban_id}",
  tag = "Bans",
  security(("Browser Session" = ["bans"])),
  params(("ban_id" = u64, Path, description = "The ban's ID")),
  responses(
    responses::Created<CreatedUnban>,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::Conflict,
  ),
)]
pub async fn unban(
	session: authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	State(state): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(unban): Json<NewUnban>,
) -> Result<Created<Json<CreatedUnban>>>
{
	state
		.submit_unban(ban_id, unban, session.user())
		.await
		.map(Json)
		.map(Created)
}
