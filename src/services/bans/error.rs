//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use super::{BanID, UnbanID};
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

	/// A player who is already banned, cannot be banned again.
	#[error("player is already banned")]
	PlayerAlreadyBanned
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific player was made, but the player could
	/// not be found.
	#[error("player with SteamID `{steam_id}` does not exist")]
	PlayerDoesNotExist
	{
		/// The player's SteamID.
		steam_id: SteamID,
	},

	/// A request dedicated to a specific ban was made, but the ban could
	/// not be found.
	#[error("ban with ID `{ban_id}` does not exist")]
	BanDoesNotExist
	{
		/// The ban's ID.
		ban_id: BanID,
	},

	/// A ban update requested the ban's expiration date to be set to a date
	/// before the ban's creation.
	#[error("ban cannot expire before it was created")]
	ExpirationBeforeCreation,

	/// A request was made to update an already reverted ban.
	#[error("ban has already been reverted")]
	BanAlreadyReverted
	{
		/// The unban's ID.
		unban_id: UnbanID,
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
			Error::Database(error) => error.into(),
			_ => todo!(),
		}
	}
}
