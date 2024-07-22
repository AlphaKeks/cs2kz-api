//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use crate::runtime;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// We have no data to return.
	#[error("no data")]
	NoData,

	/// A request dedicated to a specific user was made, but the user could not
	/// be found.
	#[error("user with SteamID `{user_id}` does not exist")]
	UserDoesNotExist
	{
		/// The user's SteamID.
		user_id: SteamID,
	},

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
			Error::NoData => Self::no_content(),
			Error::UserDoesNotExist { .. } => Self::not_found("user"),
			Error::Database(err) => err.into(),
		}
	}
}
