//! CS2KZ officially supports a set of gameplay styles that can be combined in
//! addition to a [mode].
//!
//! This module contains bitflags for these styles.
//!
//! [mode]: crate::mode

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref};
use std::str::FromStr;

use thiserror::Error;

mod iter;

#[doc(inline)]
pub use iter::Iter;

#[cfg(feature = "serde")]
mod serde;

#[cfg(feature = "sqlx")]
mod sqlx;

#[cfg(feature = "utoipa")]
mod utoipa;

/// All official gameplay styles included in the CS2KZ plugin.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Styles(u32);

impl Styles
{
	/// No styles.
	pub const NONE: Self = Self(0);

	/// The "ABH" style.
	pub const AUTO_BHOP: Self = Self(1 << 0);

	/// All styles.
	pub const ALL: Self = Self(1 << 0);

	/// Create new bitflags from a raw integer value.
	///
	/// # Panics
	///
	/// This function will panic if `value` contains any unknown bits.
	pub const fn new(value: u32) -> Self
	{
		assert!(value & Self::ALL.0 == value, "invalid style bits");
		Self(value)
	}

	/// Returns the underlying integer value.
	pub const fn bits(self) -> u32
	{
		self.0
	}

	/// If `self` currently has 1 bit set, this function will return the name
	/// of that bit.
	pub const fn name(self) -> Option<&'static str>
	{
		match self {
			Self::NONE => Some("none"),
			Self::AUTO_BHOP => Some("auto_bhop"),
			_ => None,
		}
	}

	/// Checks if `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool
	{
		(self.0 & other.0) == other.0
	}
}

impl fmt::Display for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_list().entries(self).finish()
	}
}

impl fmt::Binary for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl Deref for Styles
{
	type Target = u32;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl BitOr for Styles
{
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 | rhs.0)
	}
}

impl BitOrAssign for Styles
{
	fn bitor_assign(&mut self, rhs: Self)
	{
		*self = *self | rhs;
	}
}

impl BitAnd for Styles
{
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 & rhs.0)
	}
}

impl BitAndAssign for Styles
{
	fn bitand_assign(&mut self, rhs: Self)
	{
		*self = *self & rhs;
	}
}

impl BitXor for Styles
{
	type Output = Self;

	fn bitxor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 ^ rhs.0)
	}
}

impl BitXorAssign for Styles
{
	fn bitxor_assign(&mut self, rhs: Self)
	{
		*self = *self ^ rhs;
	}
}

/// An error that can occur when parsing a string into [`Styles`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error("unknown style `{0}`")]
pub struct UnknownStyle(pub String);

impl FromStr for Styles
{
	type Err = UnknownStyle;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"none" => Ok(Self::NONE),
			"auto_bhop" => Ok(Self::AUTO_BHOP),
			unknown => Err(UnknownStyle(unknown.to_owned())),
		}
	}
}
