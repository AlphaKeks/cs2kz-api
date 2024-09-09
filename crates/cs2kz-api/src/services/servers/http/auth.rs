use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use problem_details::AsProblemDetails;

use crate::database;
use crate::extract::Header;
use crate::http::Problem;
use crate::services::servers::models::ParseAccessKeyError;
use crate::services::servers::{AccessKey, ServerID};

#[derive(Debug, Error)]
pub enum AuthWebSocketError
{
	#[error(transparent)]
	ParseHeader(#[from] ParseAccessKeyError),

	#[error("invalid access key")]
	InvalidAccessKey,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for AuthWebSocketError
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::ParseHeader(_) => Problem::InvalidHeader,
			Self::InvalidAccessKey => Problem::Unauthorized,
			Self::Database(_) => Problem::Internal,
		}
	}
}

#[instrument(
	level = "debug",
	err(Debug, level = "debug"),
	skip(mysql, request, next)
)]
pub async fn auth_websocket(
	State(mysql): State<database::Pool>,
	Header(auth_header): Header<Authorization<Bearer>>,
	mut request: Request,
	next: Next,
) -> crate::http::Result<Response>
{
	let access_key = auth_header
		.token()
		.parse::<AccessKey>()
		.map_err(AuthWebSocketError::ParseHeader)
		.map_err(|error| error.as_problem_details())?;

	request.extensions_mut().insert(access_key);

	let server_id = get_server_id(access_key, &mysql)
		.await
		.map_err(|error| error.as_problem_details())?;

	request.extensions_mut().insert(server_id);

	Ok(next.run(request).await)
}

async fn get_server_id(
	access_key: AccessKey,
	conn: impl database::Executor<'_>,
) -> Result<ServerID, AuthWebSocketError>
{
	sqlx::query_scalar! {
		"SELECT id `server_id: ServerID`
		 FROM Servers
		 WHERE access_key = ?",
		access_key,
	}
	.fetch_optional(conn)
	.await?
	.ok_or(AuthWebSocketError::InvalidAccessKey)
}
