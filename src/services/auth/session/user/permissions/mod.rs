//! User permissions.
//!
//! Permissions are stored as bitflags, where each bit represents some
//! capability. They can be used to ensure a user has required privileges to
//! perform an action. In particular, see [`Permissions::contains()`].

use std::fmt;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Deref};
use std::str::FromStr;

use thiserror::Error;

mod iter;
pub use iter::Iter;

mod sqlx;
mod serde;

/// User permissions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Permissions(u64);

impl Permissions
{
	/// No permissions.
	pub const NONE: Self = Self(0);

	/// The user can ban/unban other players.
	pub const BANS: Self = Self(1 << 0);

	/// The user can approve and manage global servers.
	pub const SERVERS: Self = Self(1 << 8);

	/// The user can approve and manage global maps.
	pub const MAPS: Self = Self(1 << 16);

	/// The user can manage other admins.
	pub const ADMIN: Self = Self(1 << 31);

	/// All permissions.
	pub const ALL: Self = Self(1 << 0 | 1 << 8 | 1 << 16 | 1 << 31);

	/// Create new bitflags from a raw integer value.
	///
	/// # Panics
	///
	/// This function will panic if `value` contains any unknown bits.
	pub const fn new(value: u64) -> Self
	{
		assert!(value & Self::ALL.0 == value, "invalid permission bits");
		Self(value)
	}

	/// Returns the underlying integer value.
	pub const fn bits(self) -> u64
	{
		self.0
	}

	/// If `self` currently has 1 bit set, this function will return the name
	/// of that bit.
	pub const fn name(self) -> Option<&'static str>
	{
		match self {
			Self::NONE => Some("none"),
			Self::BANS => Some("bans"),
			Self::SERVERS => Some("servers"),
			Self::MAPS => Some("maps"),
			Self::ADMIN => Some("admin"),
			_ => None,
		}
	}

	/// Checks if `other` is a subset of `self`.
	pub const fn contains(self, other: Self) -> bool
	{
		(self.0 & other.0) == other.0
	}
}

impl fmt::Display for Permissions
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_list().entries(self).finish()
	}
}

impl fmt::Binary for Permissions
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Permissions
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Permissions
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for Permissions
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl Deref for Permissions
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl BitOr for Permissions
{
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 | rhs.0)
	}
}

impl BitOrAssign for Permissions
{
	fn bitor_assign(&mut self, rhs: Self)
	{
		*self = *self | rhs;
	}
}

impl BitAnd for Permissions
{
	type Output = Self;

	fn bitand(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 & rhs.0)
	}
}

impl BitAndAssign for Permissions
{
	fn bitand_assign(&mut self, rhs: Self)
	{
		*self = *self & rhs;
	}
}

impl BitXor for Permissions
{
	type Output = Self;

	fn bitxor(self, rhs: Self) -> Self::Output
	{
		Self::new(self.0 ^ rhs.0)
	}
}

impl BitXorAssign for Permissions
{
	fn bitxor_assign(&mut self, rhs: Self)
	{
		*self = *self ^ rhs;
	}
}

/// An error that can occur when parsing a string into [`Permissions`].
#[derive(Debug, Clone, PartialEq, Error)]
#[error("unknown permission `{0}`")]
pub struct UnknownPermission(pub String);

impl FromStr for Permissions
{
	type Err = UnknownPermission;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"none" => Ok(Self::NONE),
			"bans" => Ok(Self::BANS),
			"servers" => Ok(Self::SERVERS),
			"maps" => Ok(Self::MAPS),
			"admin" => Ok(Self::ADMIN),
			unknown => Err(UnknownPermission(unknown.to_owned())),
		}
	}
}
