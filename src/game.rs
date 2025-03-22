use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
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
