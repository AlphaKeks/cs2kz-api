//! The different states a global map can be in.

use std::str::FromStr;

mod errors;
pub use errors::InvalidMapState;

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

/// The different states a global map can be in.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapState
{
	/// The map is not global.
	NotGlobal = -1,

	/// The map is in a public testing phase.
	InTesting = 0,

	/// The map is global.
	Global = 1,
}

impl MapState
{
	/// Checks if this status is [Global].
	///
	/// [Global]: MapState::Global
	pub const fn is_global(&self) -> bool
	{
		matches!(self, Self::Global)
	}
}

impl From<MapState> for i8
{
	fn from(status: MapState) -> Self
	{
		status as i8
	}
}

impl TryFrom<i8> for MapState
{
	type Error = InvalidMapState;

	fn try_from(int: i8) -> Result<Self, Self::Error>
	{
		match int {
			-1 => Ok(Self::NotGlobal),
			0 => Ok(Self::InTesting),
			1 => Ok(Self::Global),
			_ => Err(InvalidMapState),
		}
	}
}

impl FromStr for MapState
{
	type Err = InvalidMapState;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = value.parse::<i8>() {
			return Self::try_from(int);
		}

		match value {
			"not_global" => Ok(Self::NotGlobal),
			"in_testing" => Ok(Self::InTesting),
			"global" => Ok(Self::Global),
			_ => Err(InvalidMapState),
		}
	}
}
