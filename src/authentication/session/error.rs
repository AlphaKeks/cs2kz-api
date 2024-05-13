//! Errors that can occur when authenticating a session.

use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::http::HandlerError;

/// Errors that can occur when authenticating a session.
#[derive(Debug, Error)]
pub enum AuthenticateSessionError {
	/// The user trying to login is not in the database.
	#[error("cannot authenticate unknown user")]
	UnknownUser {
		/// The underyling SQL error.
		source: sqlx::Error,
	},

	/// A database error.
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

impl From<AuthenticateSessionError> for HandlerError {
	fn from(error: AuthenticateSessionError) -> Self {
		let message = error.to_string();

		match error {
			AuthenticateSessionError::UnknownUser { source } => Self::unauthorized()
				.with_message(message)
				.with_source(source),
			AuthenticateSessionError::Database(source) => {
				Self::internal_server_error().with_source(source)
			}
		}
	}
}

impl IntoResponse for AuthenticateSessionError {
	fn into_response(self) -> Response {
		HandlerError::from(self).into_response()
	}
}
