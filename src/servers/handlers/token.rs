//! Handlers for the `/servers/token` route.

use axum::Json;

use crate::authentication::server::Token;
use crate::http::{HandlerError, HandlerResult};
use crate::openapi::responses::{self, Created};
use crate::plugin::PluginVersionID;
use crate::servers::{ServerID, TokenRequest};
use crate::{authentication, State};

/// Generate an access token for a CS2 server.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  tag = "Servers",
  path = "/servers/key",
  request_body = TokenRequest,
  responses(
    responses::Created<Token>,
    responses::NoContent,
    responses::BadRequest,
    responses::Unauthorized,
    responses::Conflict,
    responses::UnprocessableEntity,
  ),
)]
pub async fn generate(
	state: &State,
	Json(TokenRequest {
		key,
		plugin_version,
	}): Json<TokenRequest>,
) -> HandlerResult<Created<Token>> {
	let mut transaction = state.database.begin().await?;
	let server = sqlx::query! {
		r#"
		SELECT
		  s.id `server_id: ServerID`,
		  v.id `plugin_version_id: PluginVersionID`
		FROM
		  Servers s
		  JOIN PluginVersions v ON v.semver = ?
		  AND s.`key` = ?
		"#,
		plugin_version.to_string(),
		key,
	}
	.fetch_optional(transaction.as_mut())
	.await?
	.map(|row| authentication::Server::new(row.server_id, row.plugin_version_id))
	.ok_or_else(|| HandlerError::unauthorized())?;

	let token = Token::new(&server, state)?;

	transaction.commit().await?;

	Ok(Created(token))
}
