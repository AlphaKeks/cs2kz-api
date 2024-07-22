//! [JWT] authentication for CS2 servers.
//!
//! [JWT]: https://jwt.io

use std::fmt;
use std::ops::{Deref, DerefMut};
use std::time::Duration;

use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use axum::response::{IntoResponse, Response};
use axum::{async_trait, RequestPartsExt};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::typed_header::TypedHeaderRejection;
use axum_extra::TypedHeader;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::runtime;
use crate::services::{auth, AuthService};

mod service;
pub use service::{JwtLayer, JwtService};

/// A JWT.
///
/// This type can be used for encoding/decoding raw JWTs, and as an [extractor].
///
/// [extractor]: axum::extract
#[derive(Clone, Serialize, Deserialize)]
pub struct Jwt<T>
{
	/// The payload to encode in the token.
	#[serde(flatten)]
	payload: T,

	/// Timestamp (in seconds) of when this token will expire.
	#[serde(rename = "exp")]
	expiration_timestamp: u64,
}

impl<T> Jwt<T>
{
	/// Creates a new [`Jwt`].
	///
	/// You can encode it into a string using [`AuthService::encode_jwt()`].
	///
	/// [`AuthService::encode_jwt()`]: crate::services::AuthService::encode_jwt
	pub fn new(payload: T, expires_after: Duration) -> Self
	{
		Self {
			payload,
			expiration_timestamp: jsonwebtoken::get_current_timestamp() + expires_after.as_secs(),
		}
	}

	/// Returns a reference to the inner payload.
	pub fn payload(&self) -> &T
	{
		&self.payload
	}

	/// Returns a mutable reference to the inner payload.
	pub fn payload_mut(&mut self) -> &mut T
	{
		&mut self.payload
	}

	/// Returns the inner payload.
	pub fn into_payload(self) -> T
	{
		self.payload
	}

	/// Returns a [`chrono::DateTime`] of when this token will expire.
	pub fn expires_on(&self) -> DateTime<Utc>
	{
		let secs = i64::try_from(self.expiration_timestamp).expect("sensible expiration date");

		DateTime::from_timestamp(secs, 0).expect("valid expiration date")
	}

	/// Checks if this token has expired.
	pub fn has_expired(&self) -> bool
	{
		self.expiration_timestamp <= jsonwebtoken::get_current_timestamp()
	}
}

impl<T> fmt::Debug for Jwt<T>
where
	T: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Jwt")
			.field("payload", self.payload())
			.field("expires_on", &format_args!("{}", self.expires_on().format("%Y/%m/%d %H:%M:%S")))
			.finish()
	}
}

impl<T> Deref for Jwt<T>
{
	type Target = T;

	fn deref(&self) -> &Self::Target
	{
		self.payload()
	}
}

impl<T> DerefMut for Jwt<T>
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		self.payload_mut()
	}
}

/// Rejection for extracing a [`Jwt`] from a request.
#[derive(Debug)]
pub enum JwtRejection
{
	/// The `Authorization` header was missing / malformed.
	Header(TypedHeaderRejection),

	/// The `Authorization` header exists, but could not be decoded as a JWT.
	DecodeJwt(jsonwebtoken::errors::Error),

	/// The JWT exists, but has already expired.
	JwtExpired,

	/// The auth service failed for some reason.
	Auth(auth::Error),
}

impl IntoResponse for JwtRejection
{
	fn into_response(self) -> Response
	{
		match self {
			JwtRejection::Header(rej) => rej.into_response(),
			JwtRejection::DecodeJwt(error) => runtime::Error::bad_request(error).into_response(),
			JwtRejection::JwtExpired => {
				runtime::Error::unauthorized("token has expired").into_response()
			}
			JwtRejection::Auth(error) => runtime::Error::from(error).into_response(),
		}
	}
}

#[async_trait]
impl<T, S> FromRequestParts<S> for Jwt<T>
where
	T: DeserializeOwned + Send + Sync + 'static,
	S: Send + Sync + 'static,
	AuthService: FromRef<S>,
{
	type Rejection = JwtRejection;

	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		if let Some(jwt) = parts.extensions.remove::<Self>() {
			return Ok(jwt);
		}

		let auth_svc = AuthService::from_ref(state);

		let header = parts
			.extract::<TypedHeader<Authorization<Bearer>>>()
			.await
			.map_err(JwtRejection::Header)?;

		let jwt = auth_svc
			.decode_jwt::<T>(header.token())
			.map_err(JwtRejection::Auth)?;

		if jwt.has_expired() {
			return Err(JwtRejection::JwtExpired);
		}

		Ok(jwt)
	}
}
