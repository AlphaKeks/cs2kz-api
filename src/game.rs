use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
	#[default]
	CS2,
	CSGO,
}

impl Game
{
	/// Returns `true` if the game is [`CS2`].
	///
	/// [`CS2`]: Game::CS2
	#[must_use]
	pub fn is_cs2(&self) -> bool
	{
		matches!(self, Self::CS2)
	}

	/// Returns `true` if the game is [`CSGO`].
	///
	/// [`CSGO`]: Game::CSGO
	#[must_use]
	pub fn is_csgo(&self) -> bool
	{
		matches!(self, Self::CSGO)
	}
}

impl_rand!(Game => |rng| {
	if rng.random::<bool>() {
		Game::CS2
	} else {
		Game::CSGO
	}
});
