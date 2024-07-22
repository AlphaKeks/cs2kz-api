//! Request / Response types for this service.

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};

/// Basic information about a player.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct PlayerInfo
{
	/// The player's name.
	#[sqlx(rename = "player_name")]
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "player_id")]
	pub steam_id: SteamID,
}
