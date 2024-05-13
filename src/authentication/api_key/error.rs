//! Errors that can occur when authenticating [ApiKey]s.
//!
//! [ApiKey]: crate::authentication::ApiKey

use axum::response::{IntoResponse, Response};
use axum_extra::typed_header::TypedHeaderRejection;
use thiserror::Error;

use crate::http::HandlerError;

/// Errors that can occur when authenticating [ApiKey]s.
///
/// [ApiKey]: crate::authentication::ApiKey
#[derive(Debug, Error)]
pub enum AuthenticateApiKeyError {
	/// The key should be in an `Authorization: Bearer …` header, which might not exist.
	#[error(transparent)]
	Header(#[from] TypedHeaderRejection),

	/// The key is a UUID, so parsing it might fail.
	#[error("failed to parse API key: {0}")]
	ParseKey(#[from] uuid::Error),

	/// The key might not be in the database, or has expired.
	#[error("API key is invalid")]
	InvalidKey,

	/// A database error.
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

impl IntoResponse for AuthenticateApiKeyError {
	fn into_response(self) -> Response {
		let message = self.to_string();

		match self {
			Self::Header(rejection) => rejection.into_response(),
			Self::ParseKey(error) => HandlerError::bad_request()
				.with_message(message)
				.with_source(error)
				.into_response(),
			Self::InvalidKey => HandlerError::bad_request()
				.with_message(message)
				.into_response(),
			Self::Database(error) => HandlerError::internal_server_error()
				.with_source(error)
				.into_response(),
		}
	}
}
