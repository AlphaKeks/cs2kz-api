//! Difficulty ratings for course filters.

use std::fmt;
use std::str::FromStr;

mod errors;
pub use errors::InvalidTier;

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

/// The 10 difficulty ratings for CS2KZ course filters.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier
{
	/// The lowest tier.
	///
	/// Someone who has never played KZ before should be able to complete this.
	VeryEasy = 1,

	/// Requires some prior KZ knowledge, such as air strafing and bunnyhopping.
	Easy = 2,

	/// Players who have the KZ basics down should be able to complete this.
	Medium = 3,

	/// Players who have played KZ consistently for a while and are starting to
	/// learn more advanced techniques like ladders and surfs.
	Advanced = 4,

	/// Just like [Advanced], but harder.
	///
	/// [Advanced]: type@Tier::Advanced
	Hard = 5,

	/// Just like [Hard], but very.
	///
	/// [Hard]: type@Tier::Hard
	VeryHard = 6,

	/// For players with a lot of KZ experience who want to challenge
	/// themselves. Getting a top time on these requires mastering KZ.
	Extreme = 7,

	/// These are the hardest in the game, and only very good KZ players can
	/// complete these at all.
	Death = 8,

	/// Technically possible, but not feasible for humans. This tier is reserved
	/// for TAS runs, and any runs submitted by humans will be reviewed for
	/// cheats.
	Unfeasible = 9,

	/// Technically impossible. Even with perfect inputs.
	Impossible = 10,
}

impl Tier
{
	/// Checks if this tier is in the humanly-possible range.
	pub const fn is_humanly_possible(&self) -> bool
	{
		(*self as u8) < (Self::Unfeasible as u8)
	}
}

impl fmt::Display for Tier
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.pad(match (self, f.alternate()) {
			(Self::VeryEasy, false) => "1",
			(Self::VeryEasy, true) => "Very Easy",
			(Self::Easy, false) => "2",
			(Self::Easy, true) => "Easy",
			(Self::Medium, false) => "3",
			(Self::Medium, true) => "Medium",
			(Self::Advanced, false) => "4",
			(Self::Advanced, true) => "Advanced",
			(Self::Hard, false) => "5",
			(Self::Hard, true) => "Hard",
			(Self::VeryHard, false) => "6",
			(Self::VeryHard, true) => "Very Hard",
			(Self::Extreme, false) => "7",
			(Self::Extreme, true) => "Extreme",
			(Self::Death, false) => "8",
			(Self::Death, true) => "Death",
			(Self::Unfeasible, false) => "9",
			(Self::Unfeasible, true) => "Unfeasible",
			(Self::Impossible, false) => "10",
			(Self::Impossible, true) => "Impossible",
		})
	}
}

impl From<Tier> for u8
{
	fn from(tier: Tier) -> Self
	{
		tier as u8
	}
}

impl TryFrom<u8> for Tier
{
	type Error = InvalidTier;

	fn try_from(int: u8) -> Result<Self, Self::Error>
	{
		match int {
			1 => Ok(Self::VeryEasy),
			2 => Ok(Self::Easy),
			3 => Ok(Self::Medium),
			4 => Ok(Self::Advanced),
			5 => Ok(Self::Hard),
			6 => Ok(Self::VeryHard),
			7 => Ok(Self::Extreme),
			8 => Ok(Self::Death),
			9 => Ok(Self::Unfeasible),
			10 => Ok(Self::Impossible),
			_ => Err(InvalidTier),
		}
	}
}

impl FromStr for Tier
{
	type Err = InvalidTier;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = value.parse::<u8>() {
			return Self::try_from(int);
		}

		match value {
			"very_easy" => Ok(Self::VeryEasy),
			"easy" => Ok(Self::Easy),
			"medium" => Ok(Self::Medium),
			"advanced" => Ok(Self::Advanced),
			"hard" => Ok(Self::Hard),
			"very_hard" => Ok(Self::VeryHard),
			"extreme" => Ok(Self::Extreme),
			"death" => Ok(Self::Death),
			"unfeasible" => Ok(Self::Unfeasible),
			"impossible" => Ok(Self::Impossible),
			_ => Err(InvalidTier),
		}
	}
}
