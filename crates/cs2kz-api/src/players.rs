mod name;
pub use name::PlayerName;

mod database;
pub use database::{get, get_by_id, get_by_name, GetPlayersParams, Player};

mod register;
pub use register::{register, NewPlayer};

pub type Preferences = json::Map<String, json::Value>;

#[derive(
	Debug, Clone, Copy, PartialEq, From, Deref, serde::Serialize, serde::Deserialize, sqlx::Type,
)]
#[debug("{}", _0.as_u64())]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PlayerID(cs2kz::SteamID);
