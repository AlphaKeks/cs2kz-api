//! This module contains the [`Style`] type.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// A single style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Style
{
	/// The ABH style.
	AutoBhop = 1 << 0,
}

impl Style
{
	/// Returns an integer with exactly 1 bit set.
	///
	/// The set bit is the same bit that would be 1 in [`Styles`] for this style.
	///
	/// [`Styles`]: super::Styles
	pub const fn as_u32(self) -> u32
	{
		self as u32
	}
}

impl fmt::Display for Style
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.pad(match (self, f.alternate()) {
			(Self::AutoBhop, false) => "ABH",
			(Self::AutoBhop, true) => "Auto Bhop",
		})
	}
}

/// An error returned when parsing a string into a [`Style`].
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("unknown style")]
pub struct UnknownStyle;

impl FromStr for Style
{
	type Err = UnknownStyle;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"auto_bhop" | "abh" | "ABH" => Ok(Self::AutoBhop),
			_ => Err(UnknownStyle),
		}
	}
}

cfg_rand! {
	impl rand::distributions::Distribution<Style> for rand::distributions::Standard
	{
		fn sample<R: rand::Rng + ?Sized>(&self, _rng: &mut R) -> Style
		{
			// TODO: actually make this random once we have more than 1 style
			Style::AutoBhop
		}
	}
}

cfg_serde! {
	impl serde::Serialize for Style
	{
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
		{
			match self {
				Self::AutoBhop => "auto_bhop",
			}
			.serialize(serializer)
		}
	}

	impl<'de> serde::Deserialize<'de> for Style
	{
		fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
		where
			D: serde::Deserializer<'de>,
		{
			String::deserialize(deserializer)?
				.parse::<Self>()
				.map_err(serde::de::Error::custom)
		}
	}
}
