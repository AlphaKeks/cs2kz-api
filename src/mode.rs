use {
	crate::game::Game,
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

/// The different game modes across CS2 and CS:GO
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Mode
{
	/// VNL in CS2
	#[serde(rename = "vanilla-cs2")]
	VanillaCS2,

	/// CKZ in CS2
	Classic,

	/// KZT in CS:GO
	KZTimer,

	/// SKZ in CS:GO
	SimpleKZ,

	/// VNL in CS:GO
	#[serde(rename = "vanilla-csgo")]
	VanillaCSGO,
}

impl Mode
{
	pub const fn game(&self) -> Game
	{
		match *self {
			Mode::VanillaCS2 | Mode::Classic => Game::CS2,
			Mode::KZTimer | Mode::SimpleKZ | Mode::VanillaCSGO => Game::CSGO,
		}
	}
}
