//! Rejection types for [`Session`].
//!
//! [`Session`]: super::Session

use axum::response::{IntoResponse, Response};
use thiserror::Error;

use crate::runtime;

/// Error that can occur while authenticating a session.
#[derive(Debug, Error)]
pub enum SessionRejection
{
	/// The cookie holding the session ID is missing.
	#[error("missing session cookie")]
	MissingCookie,

	/// The session ID could not be parsed.
	#[error("invalid session id")]
	ParseSessionID(#[from] uuid::Error),

	/// The session ID was invalid.
	///
	/// This happens either because the ID is not in the database, or the
	/// session associated with that ID already expired.
	#[error("invalid session id")]
	InvalidSessionID,

	/// Something went wrong communicating with the database.
	#[error("database error; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl IntoResponse for SessionRejection
{
	fn into_response(self) -> Response
	{
		runtime::Error::from(self).into_response()
	}
}

impl From<SessionRejection> for runtime::Error
{
	#[track_caller]
	fn from(value: SessionRejection) -> Self
	{
		match value {
			reason @ (SessionRejection::MissingCookie
			| SessionRejection::ParseSessionID(_)
			| SessionRejection::InvalidSessionID) => Self::unauthorized(reason),
			SessionRejection::Database(error) => error.into(),
		}
	}
}
