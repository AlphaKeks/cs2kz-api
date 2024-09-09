//! This module implements functionality to get a specific player by their SteamID or name.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;
use serde::Serialize;

use super::PlayerService;
use crate::http::Problem;
use crate::util::net::IpAddr;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PlayerService
{
	/// Gets a specific player by their SteamID.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_player_by_id(&self, steam_id: SteamID) -> Result
	{
		let player = sqlx::query_as! {
			Response,
			"SELECT
			   u.id `steam_id: SteamID`,
			   u.name,
			   u.ip_address `ip_address: IpAddr`,
			   (
			     SELECT (COUNT(b.id) > 0)
			     FROM Bans b
			     LEFT JOIN Unbans ub ON ub.ban_id = b.id
			     WHERE b.user_id = u.id
			     AND ub.id IS NULL
			   ) `is_banned!: bool`
			 FROM Users u
			 WHERE u.id = ?",
			steam_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::PlayerNotFound)?;

		Ok(player)
	}

	/// Gets a specific player by their name.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_player_by_name(&self, name: &str) -> Result
	{
		let player = sqlx::query_as! {
			Response,
			"SELECT
			   u.id `steam_id: SteamID`,
			   u.name,
			   u.ip_address `ip_address: IpAddr`,
			   (
			     SELECT (COUNT(b.id) > 0)
			     FROM Bans b
			     LEFT JOIN Unbans ub ON ub.ban_id = b.id
			     WHERE b.user_id = u.id
			     AND ub.id IS NULL
			   ) `is_banned!: bool`
			 FROM Users u
			 WHERE u.name LIKE ?",
			format!("%{name}%"),
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::PlayerNotFound)?;

		Ok(player)
	}
}

/// Response for getting a specific player.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's name.
	pub name: String,

	/// The player's IP address.
	pub ip_address: IpAddr,

	/// Whether the player is currently banned.
	pub is_banned: bool,
}

/// Errors that can occur when getting a player.
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
