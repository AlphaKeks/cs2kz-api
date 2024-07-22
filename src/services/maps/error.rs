//! The errors that can occur when interacting with this service.

use std::io;

use thiserror::Error;

use crate::{runtime, services};

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// An operation using the steam service failed.
	#[error(transparent)]
	Steam(#[from] services::steam::Error),

	/// An I/O error occurred while calculating a map's checksum.
	#[error("failed to calculate map checksum: {0}")]
	CalculateMapChecksum(io::Error),

	#[error("one of the submitted mappers is not in the database")]
	MapperDoesNotExist,

	/// Something went wrong communicating with the database.
	#[error("database error; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl From<Error> for runtime::Error
{
	#[track_caller]
	fn from(value: Error) -> Self
	{
		match value {
			Error::Database(error) => error.into(),
			_ => todo!(),
		}
	}
}
