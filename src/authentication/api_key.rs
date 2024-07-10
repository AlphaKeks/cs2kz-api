//! Opaque API keys.
//!
//! These are one-off authentication keys used for special requests like GitHub
//! actions submitting new cs2kz plugin versions.

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use derive_more::{Debug, Display, Into};
use sqlx::{MySql, Pool};
use uuid::Uuid;

use crate::{Error, Result};

/// An opaque API key.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Into)]
#[debug("{name}")]
#[display("{key} ({name})")]
pub struct ApiKey
{
	/// The key itself.
	#[into]
	key: Uuid,

	/// The name of the service associated with this key.
	name: String,
}

impl ApiKey
{
	/// Returns this key's service's name.
	pub fn name(&self) -> &str
	{
		&self.name
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for ApiKey
where
	S: Send + Sync + 'static,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::api_key::from_request_parts",
		skip_all,
		fields(name = tracing::field::Empty, value = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(parts: &mut request::Parts, state: &S) -> Result<Self>
	{
		let key = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state)
			.await?
			.token()
			.parse::<Uuid>()
			.map_err(|err| Error::invalid("key").context(err))?;

		let database = Pool::<MySql>::from_ref(state);

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
		.fetch_optional(&database)
		.await?
		.map(|row| match row.is_expired {
			true => Err(Error::expired_key()),
			false => Ok(ApiKey { key, name: row.name }),
		})
		.ok_or_else(|| Error::unauthorized())??;

		tracing::Span::current()
			.record("name", api_key.name())
			.record("value", format_args!("{}", api_key.key));

		tracing::debug!("authenticated API key");

		Ok(api_key)
	}
}
