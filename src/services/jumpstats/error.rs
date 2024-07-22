//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use super::JumpstatID;
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

	/// A request dedicated to a specific player was made, but the player could
	/// not be found.
	#[error("player with SteamID `{steam_id}` does not exist")]
	PlayerDoesNotExist
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific jumpstat was made, but the player
	/// could not be found.
	#[error("jumpstat with ID `{jumpstat_id}` does not exist")]
	JumpstatDoesNotExist
	{
		/// The jumpstat's ID.
		jumpstat_id: JumpstatID,
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
			Error::PlayerDoesNotExist { .. } => Self::not_found("player"),
			Error::JumpstatDoesNotExist { .. } => Self::not_found("jumpstat"),
			Error::Database(error) => error.into(),
		}
	}
}
