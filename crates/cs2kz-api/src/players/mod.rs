mod name;
pub use name::PlayerName;

mod database;

#[derive(
	Debug, Clone, Copy, PartialEq, From, Deref, serde::Serialize, serde::Deserialize, sqlx::Type,
)]
#[debug("{}", _0.as_u64())]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PlayerID(cs2kz::SteamID);
