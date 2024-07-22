//! The errors that can occur when interacting with this service.

use cs2kz::SteamID;
use thiserror::Error;

use super::ServerID;
use crate::runtime;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the server service.
#[derive(Debug, Error)]
pub enum Error
{
	#[error("server owner with SteamID `{steam_id}` does not exist")]
	ServerOwnerDoesNotExist
	{
		steam_id: SteamID
	},

	#[error("server with ID `{server_id}` does not exist")]
	ServerDoesNotExist
	{
		server_id: ServerID
	},

	#[error("invalid key or plugin version")]
	InvalidKeyOrPluginVersion,

	#[error(transparent)]
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
