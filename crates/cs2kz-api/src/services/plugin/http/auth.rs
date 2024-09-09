use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use problem_details::AsProblemDetails;
use uuid::Uuid;

use crate::database;
use crate::extract::Header;
use crate::http::Problem;
use crate::util::time::Timestamp;

const KEY_NAME: &str = "github-actions:cs2kz-metamod:release";

#[instrument(err(Debug, level = "warn"), skip(mysql, request, next))]
pub async fn ensure_api_key(
	State(mysql): State<database::Pool>,
	Header(header): Header<Authorization<Bearer>>,
	request: Request,
	next: Next,
) -> Result<Response, ApiKeyRejection>
{
	let token = header.token().parse::<Uuid>()?;
	let credentials = sqlx::query! {
		"SELECT
		   name,
		   expires_on
		 FROM Credentials
		 WHERE value = ?",
		token,
	}
	.fetch_optional(&mysql)
	.await?
	.ok_or(ApiKeyRejection::UnknownToken)?;

	if credentials.name != KEY_NAME {
		warn!(credentials.name);
		return Err(ApiKeyRejection::WrongToken);
	}

	if credentials.expires_on <= Timestamp::now() {
		warn!(credentials.name, %credentials.expires_on);
		return Err(ApiKeyRejection::ExpiredToken);
	}

	Ok(next.run(request).await)
}

#[derive(Debug, Error)]
pub enum ApiKeyRejection
{
	#[error("you are not permitted to make this request")]
	MalformedToken(#[from] uuid::Error),

	#[error("you are not permitted to make this request")]
	UnknownToken,

	#[error("you are not permitted to make this request")]
	WrongToken,

	#[error("you are not permitted to make this request")]
	ExpiredToken,

	#[error("something went wrong; please report this incident")]
	DatabaseError(#[from] sqlx::Error),
}

impl AsProblemDetails for ApiKeyRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::MalformedToken(_)
			| Self::UnknownToken
			| Self::WrongToken
			| Self::ExpiredToken => Problem::Unauthorized,
			Self::DatabaseError(_) => Problem::Internal,
		}
	}
}

impl_into_response!(ApiKeyRejection);
