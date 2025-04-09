use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

/// The two games supported by the API
#[repr(u8)]
#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	Serialize,
	Deserialize,
	sqlx::Type,
	ToSchema,
	clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum Game
{
	/// Counter-Strike 2
	#[default]
	CS2,

	/// Counter-Strike: Global Offensive
	CSGO,
}

impl_rand!(Game => |rng| {
	if rng.random::<bool>() {
		Game::CS2
	} else {
		Game::CSGO
	}
});
