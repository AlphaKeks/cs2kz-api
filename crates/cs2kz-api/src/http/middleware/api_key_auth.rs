//! Authentication via opaque access tokens.
//!
//! These tokens are for internal use, such as GitHub Actions.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::middleware::Next;
use axum::response::Response;
use headers::authorization::{Authorization, Bearer};
use problem_details::AsProblemDetails;
use uuid::Uuid;

use crate::database;
use crate::http::extract::Header;
use crate::http::problem_details::Problem;
use crate::http::response::ErrorResponse;

/// State required by the API key authentication middleware.
#[derive(Debug, Clone)]
pub struct ApiKeyAuthState {
	key_name: Arc<str>,
	database: database::ConnectionPool,
}

#[derive(Debug, Error)]
#[cfg_attr(not(feature = "production"), error("invalid access key"))]
#[cfg_attr(
	feature = "production",
	error("you are not permitted to make this request")
)]
struct InvalidKey;

struct ApiKey(Uuid);
crate::database::uuid_as_bytes!(ApiKey);

/// Performs API key authentication.
#[instrument(level = "debug", skip(request, next))]
pub async fn api_key_auth(
	State(state): State<ApiKeyAuthState>,
	Header(Authorization(bearer)): Header<Authorization<Bearer>>,
	request: Request,
	next: Next,
) -> Result<Response, ErrorResponse> {
	let expected_key = state.expected().await?.ok_or_else(|| {
		error!(name = ?state.key_name, "no api key found in database");
		InvalidKey
	})?;

	if !expected_key.matches(bearer.token()) {
		debug!("token does not match");
		return Err(InvalidKey.into());
	}

	Ok(next.run(request).await)
}

impl ApiKeyAuthState {
	/// Creates a new [`ApiKeyAuthState`].
	///
	/// The given `key_name` is the name of the API key the middleware should
	/// check for.
	pub fn new(key_name: impl Into<Arc<str>>, database: database::ConnectionPool) -> Self {
		Self {
			key_name: key_name.into(),
			database,
		}
	}

	/// Fetches the expected value from the database.
	///
	/// The API key extracted from the request should [match] the returned value
	/// of this function.
	///
	/// [match]: ApiKey::matches()
	async fn expected(&self) -> database::Result<Option<ApiKey>> {
		let mut conn = self.database.get_connection().await?;

		sqlx::query_scalar!(
			"SELECT access_key `access_key: ApiKey`
			 FROM Credentials
			 WHERE name = ?
			 AND expires_at > NOW()
			 ORDER BY created_at DESC
			 LIMIT 1",
			&*self.key_name,
		)
		.fetch_optional(conn.as_mut())
		.await
		.map_err(Into::into)
	}
}

impl AsProblemDetails for InvalidKey {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::Unauthorized
	}
}

impl ApiKey {
	/// Checks if `self` matches the given `token`.
	fn matches(&self, token: &str) -> bool {
		Uuid::try_parse(token).is_ok_and(|token| token == self.0)
	}
}
