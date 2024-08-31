//! The 2 game modes available in CS2KZ.

use std::fmt;
use std::str::FromStr;

mod errors;
pub use errors::{ParseModeError, TryFromIntError};

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

/// The 2 game modes available in CS2KZ.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode
{
	/// The VNL mode.
	Vanilla = 1,

	/// The CKZ mode.
	Classic = 2,
}

impl Mode
{
	/// Checks if `self` is [Vanilla].
	///
	/// [Vanilla]: Mode::Vanilla
	pub const fn is_vanilla(&self) -> bool
	{
		matches!(self, Self::Vanilla)
	}

	/// Checks if `self` is [Classic].
	///
	/// [Classic]: Mode::Classic
	pub const fn is_classic(&self) -> bool
	{
		matches!(self, Self::Classic)
	}
}

impl fmt::Display for Mode
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.pad(match self {
			Self::Vanilla => "VNL",
			Self::Classic => "CKZ",
		})
	}
}

impl From<Mode> for u8
{
	fn from(mode: Mode) -> Self
	{
		mode as Self
	}
}

impl TryFrom<u8> for Mode
{
	type Error = TryFromIntError;

	fn try_from(value: u8) -> Result<Self, Self::Error>
	{
		match value {
			1 => Ok(Self::Vanilla),
			2 => Ok(Self::Classic),
			_ => Err(TryFromIntError),
		}
	}
}

impl FromStr for Mode
{
	type Err = ParseModeError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"1" | "vanilla" | "Vanilla" | "vnl" | "VNL" => Ok(Self::Vanilla),
			"2" | "classic" | "Classic" | "ckz" | "CKZ" => Ok(Self::Classic),
			_ => Err(ParseModeError),
		}
	}
}
