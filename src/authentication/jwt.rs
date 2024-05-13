//! Everything related to JWT authentication.
//!
//! The main attraction in this module is the [`Jwt`] type, which takes care that JWT payloads are
//! encoded / decoded correctly. It can also be used as an [extractor], to force authentication in
//! a request.
//!
//! [extractor]: axum::extract

use std::time::Duration;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum::response::{IntoResponse, Response};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::typed_header::TypedHeaderRejection;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use derive_more::{Debug, Deref, DerefMut};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::trace;

use crate::http::HandlerError;
use crate::State;

/// A JWT that can be encoded / decoded using [`State::encode_jwt()`] and
/// [`State::decode_jwt()`].
///
/// [`State::encode_jwt()`]: crate::State::encode_jwt
/// [`State::decode_jwt()`]: crate::State::decode_jwt
#[derive(Debug, Clone, Copy, Deref, DerefMut, Serialize, Deserialize)]
pub struct Jwt<T> {
	/// The encoded data.
	#[deref]
	#[deref_mut]
	#[serde(flatten)]
	pub claims: T,

	/// The expiration date.
	#[debug("{}", self.expires_on())]
	exp: u64,
}

impl<T> Jwt<T> {
	/// Creates a new [`JWT`].
	pub fn new(claims: T, expires_after: Duration) -> Self {
		Self {
			claims,
			exp: jwt::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Returns the expiration date of this token.
	///
	/// # Panics
	///
	/// This function will panic on an invalid expiration date.
	pub fn expires_on(&self) -> DateTime<Utc> {
		i64::try_from(self.exp)
			.map(|secs| DateTime::<Utc>::from_timestamp(secs, 0))
			.expect("valid expiration date")
			.expect("valid expiration date")
	}

	/// Checks whether this token has expired.
	pub fn has_expired(&self) -> bool {
		self.exp < jwt::get_current_timestamp()
	}

	/// Returns the wrapped claims.
	pub fn into_claims(self) -> T {
		self.claims
	}
}

/// The different types of errors that can occur when authenticating a JWT.
#[derive(Debug, Error)]
pub enum AuthenticateJwtError {
	/// The header was missing / invalid.
	#[error(transparent)]
	Header(#[from] TypedHeaderRejection),

	/// The header was present but not a valid JWT.
	#[error(transparent)]
	InvalidToken(#[from] jwt::errors::Error),

	/// The JWT has expired.
	#[error("JWT has expired; please request a new one")]
	Expired,
}

impl IntoResponse for AuthenticateJwtError {
	fn into_response(self) -> Response {
		match self {
			Self::Header(rejection) => rejection.into_response(),
			Self::InvalidToken(error) => HandlerError::bad_request()
				.with_message(error.to_string())
				.into_response(),
			Self::Expired => HandlerError::unauthorized()
				.with_message(self.to_string())
				.into_response(),
		}
	}
}

#[async_trait]
impl<T> FromRequestParts<&'static State> for Jwt<T>
where
	T: DeserializeOwned,
{
	type Rejection = AuthenticateJwtError;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self, Self::Rejection> {
		let header = TypedHeader::<Authorization<Bearer>>::from_request_parts(parts, state).await?;
		let jwt = state.decode_jwt::<_, T>(header.token())?;

		if jwt.has_expired() {
			return Err(AuthenticateJwtError::Expired);
		}

		trace!(token = %header.token(), "authenticated JWT");

		Ok(jwt)
	}
}
