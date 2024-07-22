//! The errors that can occur when interacting with this service.

use thiserror::Error;

use crate::{runtime, setup};

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the auth service.
#[derive(Debug, Error)]
pub enum Error
{
	#[error("failed to encode jwt")]
	EncodeJwt(jsonwebtoken::errors::Error),

	#[error("failed to decode jwt: {0}")]
	DecodeJwt(jsonwebtoken::errors::Error),

	/// Something went wrong communicating with the database.
	#[error(transparent)]
	Database(#[from] sqlx::Error),
}

impl From<Error> for runtime::Error
{
	#[track_caller]
	fn from(value: Error) -> Self
	{
		match value {
			Error::Database(err) => err.into(),
			_ => todo!(),
		}
	}
}

/// The errors that can occur when setting up the auth service.
#[derive(Debug, Error)]
pub enum SetupError
{
	/// Something went wrong setting up the JWT state.
	#[error("failed to setup jwt state: {0}")]
	SetupJwtState(#[from] jsonwebtoken::errors::Error),
}

impl From<SetupError> for setup::Error
{
	fn from(value: SetupError) -> Self
	{
		match value {
			SetupError::SetupJwtState(error) => error.into(),
		}
	}
}
