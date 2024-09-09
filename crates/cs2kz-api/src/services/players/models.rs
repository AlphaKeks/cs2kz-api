use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};

use crate::database;

/// A player's in-game preferences.
pub type Preferences = serde_json::Map<String, serde_json::Value>;

/// A player ID or name.
#[derive(Debug, Clone)]
pub enum PlayerIdentifier
{
	/// A SteamID.
	ID(SteamID),

	/// A name.
	Name(String),
}

impl PlayerIdentifier
{
	/// Returns the SteamID contained in `self` or fetches it from the database by looking up
	/// the name.
	pub async fn resolve_id(
		&self,
		conn: impl database::Executor<'_>,
	) -> database::Result<Option<SteamID>>
	{
		match *self {
			Self::ID(steam_id) => Ok(Some(steam_id)),
			Self::Name(ref name) => {
				sqlx::query_scalar! {
					"SELECT id `steam_id: SteamID`
					 FROM Users
					 WHERE name LIKE ?",
					format!("%{name}%"),
				}
				.fetch_optional(conn)
				.await
			}
		}
	}
}

impl<'de> Deserialize<'de> for PlayerIdentifier
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		pub enum Helper
		{
			SteamID(SteamID),
			Str(String),
		}

		Helper::deserialize(deserializer).map(|v| match v {
			Helper::SteamID(steam_id) => Self::ID(steam_id),
			Helper::Str(str) => str
				.parse::<SteamID>()
				.map_or_else(|_| Self::Name(str), Self::ID),
		})
	}
}

/// Information about a KZ player.
#[derive(Debug, Serialize, Deserialize)]
pub struct PlayerInfo
{
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	pub steam_id: SteamID,
}
