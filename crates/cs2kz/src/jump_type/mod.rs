//! Jumpstat types.

use std::fmt;
use std::str::FromStr;

mod errors;
pub use errors::InvalidJumpType;

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

/// The different types of jumps tracked by jumpstats.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JumpType
{
	/// LJ
	LongJump = 1,

	/// BH
	Bhop = 2,

	/// MBH
	MultiBhop = 3,

	/// WJ
	WeirdJump = 4,

	/// LAJ
	LadderJump = 5,

	/// LAH
	Ladderhop = 6,

	/// JB
	Jumpbug = 7,
}

impl fmt::Display for JumpType
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.pad(match self {
			Self::LongJump => "LJ",
			Self::Bhop => "BH",
			Self::MultiBhop => "MBH",
			Self::WeirdJump => "WJ",
			Self::LadderJump => "LAJ",
			Self::Ladderhop => "LAH",
			Self::Jumpbug => "JB",
		})
	}
}

impl From<JumpType> for u8
{
	fn from(jump_type: JumpType) -> Self
	{
		jump_type as u8
	}
}

impl TryFrom<u8> for JumpType
{
	type Error = InvalidJumpType;

	fn try_from(int: u8) -> Result<Self, Self::Error>
	{
		match int {
			1 => Ok(Self::LongJump),
			2 => Ok(Self::Bhop),
			3 => Ok(Self::MultiBhop),
			4 => Ok(Self::WeirdJump),
			5 => Ok(Self::LadderJump),
			6 => Ok(Self::Ladderhop),
			7 => Ok(Self::Jumpbug),
			_ => Err(InvalidJumpType),
		}
	}
}

impl FromStr for JumpType
{
	type Err = InvalidJumpType;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = value.parse::<u8>() {
			return Self::try_from(int);
		}

		match value {
			"lj" | "long_jump" => Ok(Self::LongJump),
			"bh" | "bhop" => Ok(Self::Bhop),
			"mbh" | "multi_bhop" => Ok(Self::MultiBhop),
			"wj" | "weird_jump" => Ok(Self::WeirdJump),
			"laj" | "ladder_jump" => Ok(Self::LadderJump),
			"lah" | "ladder_hop" => Ok(Self::Ladderhop),
			"jb" | "jumpbug" => Ok(Self::Jumpbug),
			_ => Err(InvalidJumpType),
		}
	}
}
