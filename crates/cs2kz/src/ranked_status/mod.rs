//! The different states a course filter can be in.

use std::str::FromStr;

mod errors;
pub use errors::InvalidRankedStatus;

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

/// The different states a course filter can be in.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RankedStatus
{
	/// The filter will never be [ranked], as per the mapper's request.
	///
	/// [ranked]: RankedStatus::Ranked
	Never = -1,

	/// The filter is currently not ranked, but not because it was explicitly
	/// requested, just because it didn't meet requirements.
	Unranked = 0,

	/// The filter is ranked.
	Ranked = 1,
}

impl RankedStatus
{
	/// Checks if this status is [Ranked].
	///
	/// [Ranked]: RankedStatus::Ranked
	pub const fn is_ranked(&self) -> bool
	{
		matches!(self, Self::Ranked)
	}
}

impl From<RankedStatus> for i8
{
	fn from(status: RankedStatus) -> Self
	{
		status as i8
	}
}

impl TryFrom<i8> for RankedStatus
{
	type Error = InvalidRankedStatus;

	fn try_from(int: i8) -> Result<Self, Self::Error>
	{
		match int {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			_ => Err(InvalidRankedStatus),
		}
	}
}

impl FromStr for RankedStatus
{
	type Err = InvalidRankedStatus;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(int) = value.parse::<i8>() {
			return Self::try_from(int);
		}

		match value {
			"never" => Ok(Self::Never),
			"unranked" => Ok(Self::Unranked),
			"ranked" => Ok(Self::Ranked),
			_ => Err(InvalidRankedStatus),
		}
	}
}
