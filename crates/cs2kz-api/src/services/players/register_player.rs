//! This module implements functionality to register new players.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;
use serde::Deserialize;

use super::{get_player, PlayerService};
use crate::database::ErrorExt;
use crate::http::Problem;
use crate::util::net::IpAddr;
use crate::util::NonEmpty;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PlayerService
{
	/// Registers a new player.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn register_player(&self, request: Request) -> Result
	{
		sqlx::query! {
			"INSERT INTO Users
			   (id, name, ip_address)
			 VALUES
			   (?, ?, ?)",
			request.steam_id,
			&*request.name,
			request.ip_address,
		}
		.execute(&self.mysql)
		.await
		.map_err(|error| {
			if error.is_duplicate() {
				Error::PlayerAlreadyExists
			} else {
				Error::Database(error)
			}
		})?;

		Ok(get_player::Response {
			steam_id: request.steam_id,
			name: NonEmpty::into_inner(request.name),
			ip_address: request.ip_address,
			is_banned: false,
		})
	}

	/// Registers a new player or returns their information if they're already registered.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn register_or_get_player(
		&self,
		request: Request,
	) -> sqlx::Result<get_player::Response>
	{
		let steam_id = request.steam_id;

		match self.register_player(request).await {
			Ok(response) => Ok(response),
			Err(Error::Database(error)) => Err(error),
			Err(Error::PlayerAlreadyExists) => match self.get_player_by_id(steam_id).await {
				Ok(response) => Ok(response),
				Err(get_player::Error::Database(error)) => Err(error),
				Err(get_player::Error::PlayerNotFound) => unreachable!(),
			},
		}
	}
}

/// Request for registering a new player.
#[derive(Debug, Clone, Deserialize)]
pub struct Request
{
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's name.
	pub name: NonEmpty<String>,

	/// The player's IP address.
	pub ip_address: IpAddr,
}

/// Response for registering a new player.
pub type Response = get_player::Response;

/// Errors that can occur when registering a new player.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("player already exists")]
	PlayerAlreadyExists,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::PlayerAlreadyExists => Problem::ResourceAlreadyExists,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
