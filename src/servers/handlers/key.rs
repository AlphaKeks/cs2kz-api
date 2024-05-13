//! Handlers for the `/servers/{server}/key` route.

use axum::extract::Path;
use axum::Json;
use tracing::info;

use crate::authorization::Permissions;
use crate::http::{HandlerError, HandlerResult};
use crate::openapi::responses::{Created, NoContent};
use crate::openapi::{parameters, responses};
use crate::servers::{CreatedServerKey, ServerID};
use crate::{authentication, authorization, State};

/// Replace a server's API key.
///
/// This can be used by global admins as well as server owners.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  put,
  tag = "Servers",
  path = "/servers/{server_id}/key",
  params(parameters::ServerID),
  responses(
    responses::Created<CreatedServerKey>,
    responses::BadRequest,
    responses::Unauthorized,
  ),
)]
pub async fn replace(
	state: &State,
	session: authentication::Session<authorization::IsServerAdminOrOwner>,
	Path(server_id): Path<ServerID>,
) -> HandlerResult<Created<Json<CreatedServerKey>>> {
	let mut transaction = state.database.begin().await?;
	let key = authentication::server::Key::new();
	let was_updated = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  `key` = ?
		WHERE
		  id = ?
		"#,
		key,
		server_id,
	}
	.execute(transaction.as_mut())
	.await
	.map(|result| result.rows_affected() > 0)?;

	assert!(
		was_updated,
		"session extractor should have ensured this is a known server"
	);

	transaction.commit().await?;

	info! {
		target: "audit_log",
		%server_id,
		%key,
		updated_by = ?session.user(),
		"generated new key for server",
	};

	Ok(Created(Json(CreatedServerKey { key })))
}

/// Delete a server's API key.
///
/// This can only be used by global admins.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
	delete,
	tag = "Servers",
	path = "/servers/{server_id}/key",
	params(parameters::ServerID),
	responses(responses::NoContent, responses::BadRequest, responses::Unauthorized,)
)]
pub async fn delete(
	state: &State,
	session: authentication::Session<
		authorization::HasPermissions<{ Permissions::SERVERS.value() }>,
	>,
	Path(server_id): Path<ServerID>,
) -> HandlerResult<NoContent> {
	let mut transaction = state.database.begin().await?;
	let was_deleted = sqlx::query! {
		r#"
		UPDATE
		  Servers
		SET
		  `key` = NULL
		WHERE
		  id = ?
		"#,
		server_id,
	}
	.execute(transaction.as_mut())
	.await
	.map(|result| result.rows_affected() > 0)?;

	if !was_deleted {
		return Err(HandlerError::unknown("server"));
	}

	transaction.commit().await?;

	info!(target: "audit_log", %server_id, admin = ?session.user(), "deleted key of server");

	Ok(NoContent)
}
