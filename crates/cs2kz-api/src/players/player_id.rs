use std::fmt;
use std::str::FromStr;

use cs2kz::SteamID;

/// An ID uniquely identifying a player.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(value_type = u64)]
pub struct PlayerID(SteamID);

impl fmt::Debug for PlayerID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_tuple("PlayerID").field(&*self.0).finish()
	}
}

impl FromStr for PlayerID {
	type Err = <SteamID as FromStr>::Err;

	fn from_str(str: &str) -> Result<Self, Self::Err> {
		str.parse::<SteamID>().map(Self)
	}
}
