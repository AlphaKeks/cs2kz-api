//! Handlers for the `/servers/{server}` route.

use axum::extract::Path;
use axum::Json;
use cs2kz::ServerIdentifier;
use sqlx::QueryBuilder;
use tracing::info;

use crate::database::UpdateQuery;
use crate::http::{HandlerError, HandlerResult};
use crate::openapi::responses::NoContent;
use crate::openapi::{parameters, responses};
use crate::servers::{queries, Server, ServerID, ServerUpdate};
use crate::{authentication, authorization, State};

/// Fetch a specific server by its name or ID.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Servers",
  path = "/servers/{server}",
  params(parameters::ServerIdentifier),
  responses(
    responses::Ok<Server>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: &State,
	Path(server): Path<ServerIdentifier>,
) -> HandlerResult<Json<Server>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	match server {
		ServerIdentifier::ID(id) => {
			query.push("s.id = ").push_bind(id);
		}
		ServerIdentifier::Name(name) => {
			query.push("s.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let server = query
		.build_query_as::<Server>()
		.fetch_optional(&state.database)
		.await?
		.ok_or_else(|| HandlerError::no_content())?;

	Ok(Json(server))
}

/// Update a server.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  tag = "Servers",
  path = "/servers/{server_id}",
  params(parameters::ServerID),
  request_body = ServerUpdate,
  responses(
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn patch(
	state: &State,
	session: authentication::Session<authorization::IsServerAdminOrOwner>,
	Path(server_id): Path<ServerID>,
	Json(update): Json<ServerUpdate>,
) -> HandlerResult<NoContent> {
	if update.is_empty() {
		return Ok(NoContent);
	}

	let mut transaction = state.database.begin().await?;
	let mut query = UpdateQuery::new("Servers");

	if let Some(name) = update.name.as_deref() {
		query.set("name", name);
	}

	if let Some(ip_address) = update.ip_address {
		query
			.set("ip_address", ip_address.ip().to_string())
			.set("port", ip_address.port());
	}

	if let Some(owned_by) = update.owned_by {
		query.set("owned_by", owned_by);
	}

	query.push(" WHERE id = ").push_bind(server_id);

	let was_updated = query
		.build()
		.execute(transaction.as_mut())
		.await
		.map(|result| result.rows_affected() > 0)?;

	if !was_updated {
		return Err(HandlerError::unknown("server"));
	}

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, updated_by = ?session.user(), "updated server");

	Ok(NoContent)
}
