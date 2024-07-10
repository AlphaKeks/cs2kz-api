//! HTTP handlers for the `/servers` endpoint.

use axum::extract::{Path, State};
use axum::http::Method;
use axum::{routing, Json, Router};
use axum_extra::extract::Query;

use super::{
	AccessKeyRequest,
	AccessKeyResponse,
	CreatedServer,
	FetchServersRequest,
	NewServer,
	RefreshKey,
	Server,
	ServerID,
	ServerService,
	ServerUpdate,
};
use crate::authentication::Jwt;
use crate::authorization::{self, Permissions};
use crate::kz::ServerIdentifier;
use crate::middleware::auth::session_auth;
use crate::middleware::cors;
use crate::openapi::responses::{self, Created, NoContent, PaginationResponse};
use crate::{authentication, Result};

impl From<ServerService> for Router
{
	fn from(state: ServerService) -> Self
	{
		let is_admin = session_auth!(
			authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
			state.clone(),
		);

		let is_admin_or_owner = session_auth!(authorization::IsServerAdminOrOwner, state.clone());

		let root = Router::new()
			.route("/", routing::get(get_many))
			.route_layer(cors::permissive())
			.route("/", routing::post(register).route_layer(is_admin()))
			.route_layer(cors::dashboard([Method::POST]))
			.with_state(state.clone());

		let key = Router::new()
			.route("/key", routing::post(generate_token))
			.with_state(state.clone());

		let by_identifier = Router::new()
			.route("/:server", routing::get(get_single))
			.route_layer(cors::permissive())
			.route("/:server", routing::patch(update).route_layer(is_admin_or_owner()))
			.route_layer(cors::dashboard([Method::PATCH]))
			.with_state(state.clone());

		let by_identifier_key = Router::new()
			.route("/:server/key", routing::put(replace_key).route_layer(is_admin_or_owner()))
			.route("/:server/key", routing::delete(delete_key).route_layer(is_admin()))
			.route_layer(cors::dashboard([Method::PUT, Method::DELETE]))
			.with_state(state.clone());

		root.merge(key)
			.merge(by_identifier)
			.merge(by_identifier_key)
	}
}

/// Fetch servers.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/servers",
  tag = "Servers",
  params(FetchServersRequest),
  responses(
    responses::Ok<PaginationResponse<Server>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get_many(
	State(state): State<ServerService>,
	Query(request): Query<FetchServersRequest>,
) -> Result<Json<PaginationResponse<Server>>>
{
	state
		.fetch_servers(request)
		.await
		.map(|(servers, total)| PaginationResponse { total, results: servers })
		.map(Json)
}

/// Fetch a server by its name or ID.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/servers/{server}",
  tag = "Servers",
  responses(
    responses::Ok<Server>,
    responses::BadRequest,
    responses::NotFound,
  ),
)]
pub async fn get_single(
	State(state): State<ServerService>,
	Path(server): Path<ServerIdentifier>,
) -> Result<Json<Server>>
{
	state.fetch_server(server).await.map(Json)
}

/// Create a new server.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/servers",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  responses(
    responses::Created<CreatedServer>,
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn register(
	session: authentication::Session<
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
	>,
	State(state): State<ServerService>,
	Json(server): Json<NewServer>,
) -> Result<Created<Json<CreatedServer>>>
{
	state.register_server(server).await.map(Json).map(Created)
}

/// Update an existing server.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  patch,
  path = "/servers/{server}",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn update(
	session: authentication::Session<authorization::IsServerAdminOrOwner>,
	State(state): State<ServerService>,
	Path(server_id): Path<ServerID>,
	Json(update): Json<ServerUpdate>,
) -> Result<NoContent>
{
	if update.is_empty() {
		return Ok(NoContent);
	}

	state
		.update_server(server_id, update)
		.await
		.map(|()| NoContent)
}

/// Generate a temporary access token using a CS2 server's API key.
///
/// This endpoint is for CS2 servers. They will generate a new access token
/// every ~30min.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  post,
  path = "/servers/key",
  tag = "Servers",
  responses(
    responses::Created<Jwt<authentication::Server>>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn generate_token(
	State(state): State<ServerService>,
	Json(request): Json<AccessKeyRequest>,
) -> Result<Created<Json<AccessKeyResponse>>>
{
	state
		.generate_access_token(request)
		.await
		.map(Json)
		.map(Created)
}

/// Generate a new API key for a server, invalidating the old one.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  put,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
  ),
)]
pub async fn replace_key(
	session: authentication::Session<authorization::IsServerAdminOrOwner>,
	State(state): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<Created<Json<RefreshKey>>>
{
	state
		.replace_api_key(server_id)
		.await
		.map(Json)
		.map(Created)
}

/// Delete a server's API key, preventing them from generating new JWTs.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  delete,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  security(("Browser Session" = ["servers"])),
  params(("server_id" = u16, Path, description = "The server's ID")),
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::NotFound,
    responses::Unauthorized,
  ),
)]
pub async fn delete_key(
	session: authentication::Session<
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
	>,
	State(state): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<NoContent>
{
	state.delete_api_key(server_id).await.map(|()| NoContent)
}
