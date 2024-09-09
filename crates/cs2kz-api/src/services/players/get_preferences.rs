//! This module implements functionality to get a player's in-game preferences.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;

use super::{PlayerService, Preferences};
use crate::database;
use crate::http::Problem;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PlayerService
{
	/// Gets a player's in-game preferences.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_player_preferences(&self, steam_id: SteamID) -> Result
	{
		let preferences = sqlx::query_scalar! {
			"SELECT game_preferences `game_preferences: database::Json<Preferences>`
			 FROM Users
			 WHERE id = ?",
			steam_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::PlayerNotFound)?;

		Ok(preferences.0)
	}
}

/// Response for getting a player's preferences.
pub type Response = Preferences;

/// Errors that can occur when getting a player's preferences.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("player not found")]
	PlayerNotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::PlayerNotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
