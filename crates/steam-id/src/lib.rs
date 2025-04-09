#![doc = include_str!("../README.md")]
#![feature(unqualified_local_imports)]
#![feature(non_exhaustive_omitted_patterns_lint)]

#[macro_use(Debug, Display, From, Error, FromStr)]
extern crate derive_more as _;

pub use self::{
	account_instance::AccountInstance,
	account_number::AccountNumber,
	account_type::{AccountType, InvalidAccountType},
	account_universe::{AccountUniverse, InvalidAccountUniverse},
	error::{InvalidSteamId64, ParseSteam2IdError, ParseSteamIdError},
};
use std::{borrow::Borrow, fmt, ops::Deref, str::FromStr};

mod account_instance;
mod account_number;
mod account_type;
mod account_universe;
mod error;

#[cfg(feature = "serde")]
mod serde_impls;

#[cfg(feature = "rand")]
mod rand_impls;

/// A [SteamID]
///
/// [SteamID]: https://developer.valvesoftware.com/wiki/SteamID
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamId(u64);

impl SteamId
{
	/// Returns the 64-bit representation of this [`SteamId`].
	pub const fn as_u64(&self) -> u64
	{
		self.0
	}

	/// Returns the universe of the account this SteamID belongs to.
	pub const fn account_universe(&self) -> AccountUniverse
	{
		match AccountUniverse::from_bits(self.0) {
			Some(universe) => universe,
			None => panic!("invalid universe bits in SteamID"),
		}
	}

	/// Returns the type of account this SteamID belongs to.
	pub const fn account_type(&self) -> AccountType
	{
		match AccountType::from_bits(self.0) {
			Some(universe) => universe,
			None => panic!("invalid account type bits in SteamID"),
		}
	}

	/// Returns the account instance this SteamID belongs to.
	pub const fn account_instance(&self) -> AccountInstance
	{
		AccountInstance::from_bits(self.0)
	}

	/// Returns the account number of the account this SteamID belongs to.
	pub const fn account_number(&self) -> AccountNumber
	{
		AccountNumber::from_bits(self.0)
	}

	/// Returns whether the 'Y' (least significant) bit is set.
	pub const fn y(&self) -> bool
	{
		(self.0 & 1) == 1
	}

	/// Constructs a new [`SteamId`] from its individual components.
	///
	/// This constructor assumes the account type to be [`Individual`] and the [default `Instance`].
	///
	/// [`Individual`]: AccountType::Individual
	/// [default `Instance`]: AccountInstance::DEFAULT
	pub const fn from_parts(
		universe: AccountUniverse,
		account_number: AccountNumber,
		y: bool,
	) -> Self
	{
		let bits = ((universe as u64) << 56_u64)
			| ((AccountType::Individual as u64) << 52_u64)
			| ((AccountInstance::DEFAULT.raw() as u64) << 32_u64)
			| ((account_number.raw() as u64) << 1_u64)
			| (y as u64);

		Self(bits)
	}

	/// Creates a [`SteamId`] from its raw 64-bit representation.
	pub const fn from_u64(value: u64) -> Result<Self, InvalidSteamId64>
	{
		if AccountUniverse::from_bits(value).is_none() {
			return Err(InvalidSteamId64::InvalidUniverse);
		}

		if AccountType::from_bits(value).is_none() {
			return Err(InvalidSteamId64::InvalidAccountType);
		}

		Ok(Self(value))
	}

	/// Parses a string assuming the Steam2ID format.
	pub fn parse_id2(input: &str) -> Result<Self, ParseSteam2IdError>
	{
		let mut segments = input
			.strip_prefix("STEAM_")
			.ok_or(ParseSteam2IdError::MissingPrefix)?
			.splitn(3, ':');

		let universe = segments
			.next()
			.ok_or(ParseSteam2IdError::MissingX)?
			.parse::<AccountUniverse>()?;

		let y = match segments.next().ok_or(ParseSteam2IdError::MissingY)? {
			"0" => false,
			"1" => true,
			_ => return Err(ParseSteam2IdError::InvalidY),
		};

		let account_number = segments
			.next()
			.ok_or(ParseSteam2IdError::MissingZ)?
			.parse::<AccountNumber>()?;

		Ok(Self::from_parts(universe, account_number, y))
	}
}

impl fmt::Display for SteamId
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		if fmt.alternate() {
			write!(fmt, "U:1:{}", (self.account_number().raw() * 2) + u32::from(self.y()))
		} else {
			write!(
				fmt,
				"STEAM_{}:{}:{}",
				self.account_universe() as u8,
				u8::from(self.y()),
				self.account_number()
			)
		}
	}
}

impl Borrow<u64> for SteamId
{
	fn borrow(&self) -> &u64
	{
		&self.0
	}
}

impl AsRef<u64> for SteamId
{
	fn as_ref(&self) -> &u64
	{
		self.borrow()
	}
}

impl Deref for SteamId
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		self.borrow()
	}
}

impl TryFrom<u64> for SteamId
{
	type Error = InvalidSteamId64;

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		Self::from_u64(value)
	}
}

impl FromStr for SteamId
{
	type Err = ParseSteamIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(value) = value.parse::<u64>() {
			return Ok(Self::from_u64(value)?);
		}

		Err(ParseSteamIdError::UnknownFormat)
	}
}
