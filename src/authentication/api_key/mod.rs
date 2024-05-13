//! Everything related to API key authentication.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display, Into};
use tracing::debug;
use uuid::Uuid;

use crate::State;

mod error;

#[doc(inline)]
pub use error::AuthenticateApiKeyError;

/// An opaque API key.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Into)]
#[debug("{name}")]
#[display("{key} ({name})")]
pub struct ApiKey {
	/// The secret key.
	#[into]
	key: Uuid,

	/// The name of the key.
	name: String,
}

impl ApiKey {
	/// Generate a new [`ApiKey`].
	pub fn new<S>(name: S) -> Self
	where
		S: Into<String>,
	{
		Self {
			key: Uuid::new_v4(),
			name: name.into(),
		}
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for ApiKey {
	type Rejection = AuthenticateApiKeyError;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self, Self::Rejection> {
		let key = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await?
			.token()
			.parse::<Uuid>()?;

		let api_key = sqlx::query! {
			r#"
			SELECT
			  name,
			  COALESCE((expires_on < NOW()), FALSE) `is_expired!: bool`
			FROM
			  Credentials
			WHERE
			  `key` = ?
			"#,
			key,
		}
		.fetch_optional(&state.database)
		.await?
		.filter(|row| !row.is_expired)
		.map(|row| ApiKey {
			key,
			name: row.name,
		})
		.ok_or(AuthenticateApiKeyError::InvalidKey)?;

		debug!(?api_key, "authenticated API key");

		Ok(api_key)
	}
}
