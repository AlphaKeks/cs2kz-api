//! JWT authentication.
//!
//! This module contains the [`Jwt`] type, which is used by
//! [`JwtState::encode()`] / [`JwtState::decode()`], and can be used as an
//! [extractor].
//!
//! [extractor]: axum::extract

use std::panic::Location;
use std::sync::Arc;
use std::time::Duration;

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use derive_more::{Debug, Deref, DerefMut};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use utoipa::openapi::schema::Schema;
use utoipa::openapi::{ObjectBuilder, RefOr, SchemaType};
use utoipa::ToSchema;

use crate::{Error, Result};

/// JWT state for encoding/decoding tokens.
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct JwtState
{
	jwt_header: jwt::Header,
	jwt_encoding_key: jwt::EncodingKey,
	jwt_decoding_key: jwt::DecodingKey,
	jwt_validation: jwt::Validation,
}

impl JwtState
{
	/// Creates a new [`JwtState`].
	pub fn new(api_config: &crate::Config) -> Result<Self>
	{
		let jwt_header = jwt::Header::default();

		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&api_config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&api_config.jwt_secret)
			.map_err(|err| Error::encode_jwt(err))?;

		let jwt_validation = jwt::Validation::default();

		Ok(Self { jwt_header, jwt_encoding_key, jwt_decoding_key, jwt_validation })
	}

	/// Encodes a JWT.
	pub fn encode<T>(&self, jwt: Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jwt::encode(&self.jwt_header, &jwt, &self.jwt_encoding_key)
			.map_err(|err| Error::encode_jwt(err))
	}

	/// Decodes a JWT.
	pub fn decode<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		jwt::decode(jwt, &self.jwt_decoding_key, &self.jwt_validation)
			.map(|jwt| jwt.claims)
			.map_err(|err| Error::invalid("jwt").context(err))
	}
}

/// An extractor for JWTs.
#[derive(Debug, Deref, DerefMut, Serialize, Deserialize)]
pub struct Jwt<T>
{
	/// The token payload.
	#[serde(flatten)]
	#[deref]
	#[deref_mut]
	#[debug("{payload:?}")]
	pub payload: T,

	/// The token's expiration date, as a unix timestamp.
	#[debug("{}", self.expires_on())]
	exp: u64,
}

impl<T> Jwt<T>
{
	/// Creates a new JWT from the given `payload`, that will expire after the
	/// specified duration.
	#[track_caller]
	#[tracing::instrument(
		level = "debug",
		name = "authentication::jwt::new",
		skip(payload),
		fields(location = %Location::caller()),
	)]
	pub fn new(payload: T, expires_after: Duration) -> Self
	{
		Self { payload, exp: jwt::get_current_timestamp() + expires_after.as_secs() }
	}

	/// Returns a timestamp of when this JWT will expire.
	pub fn expires_on(&self) -> DateTime<Utc>
	{
		let secs = i64::try_from(self.exp).expect("invalid expiration date");

		DateTime::from_timestamp(secs, 0).expect("invalid expiration date")
	}

	/// Checks if this JWT has expired.
	pub fn has_expired(&self) -> bool
	{
		self.exp < jwt::get_current_timestamp()
	}

	/// Turns this JWT into its inner payload.
	pub fn into_payload(self) -> T
	{
		self.payload
	}
}

#[async_trait]
impl<S, T> FromRequestParts<S> for Jwt<T>
where
	S: Send + Sync + 'static,
	T: DeserializeOwned,
	Arc<JwtState>: FromRef<S>,
{
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "auth::jwt::from_request_parts",
		skip_all,
		fields(token = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(parts: &mut request::Parts, state: &S) -> Result<Self>
	{
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let state = Arc::<JwtState>::from_ref(state);
		let jwt = state
			.decode::<T>(header.token())
			.map_err(|err| Error::invalid("token").context(err))?;

		if jwt.has_expired() {
			return Err(Error::expired_key());
		}

		tracing::Span::current().record("token", header.token());
		tracing::debug!("authenticated JWT");

		Ok(jwt)
	}
}

impl<'s, T> ToSchema<'s> for Jwt<T>
{
	fn schema() -> (&'s str, RefOr<Schema>)
	{
		(
			"JWT",
			ObjectBuilder::new()
				.description(Some("https://jwt.io"))
				.schema_type(SchemaType::String)
				.build()
				.into(),
		)
	}
}
