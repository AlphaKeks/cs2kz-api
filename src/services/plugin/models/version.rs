//! Custom wrapper around [`semver::Version`] so we can override trait
//! implementations.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A CS2KZ plugin version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, utoipa::ToSchema)]
#[schema(value_type = str, example = "0.0.1")]
pub struct PluginVersion(Repr);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Repr
{
	SemVer(semver::Version),

	#[cfg(not(feature = "production"))]
	Dev,
}

impl fmt::Display for PluginVersion
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match &self.0 {
			Repr::SemVer(version) => fmt::Display::fmt(version, f),

			#[cfg(not(feature = "production"))]
			Repr::Dev => f.pad("dev"),
		}
	}
}

impl FromStr for PluginVersion
{
	type Err = <semver::Version as FromStr>::Err;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		#[cfg(not(feature = "production"))]
		{
			if s == "dev" {
				return Ok(Self(Repr::Dev));
			}
		}

		s.parse().map(Repr::SemVer).map(Self)
	}
}

impl Serialize for PluginVersion
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match &self.0 {
			Repr::SemVer(version) => version.serialize(serializer),

			#[cfg(not(feature = "production"))]
			Repr::Dev => "dev".serialize(serializer),
		}
	}
}

impl<'de> Deserialize<'de> for PluginVersion
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut s = &*String::deserialize(deserializer)?;

		#[cfg(not(feature = "production"))]
		{
			if s == "dev" {
				return Ok(Self(Repr::Dev));
			}
		}

		if s.starts_with('v') {
			s = &s[1..];
		}

		s.parse::<semver::Version>()
			.map(Repr::SemVer)
			.map(Self)
			.map_err(serde::de::Error::custom)
	}
}

crate::macros::sqlx_scalar_forward!(PluginVersion as String => {
	encode: |self| { self.to_string() },
	decode: |value| {
		#[cfg(not(feature = "production"))]
		{
			if value == "dev" {
				return Ok(Self(Repr::Dev));
			}
		}

		value.parse::<semver::Version>().map(Repr::SemVer).map(Self)?
	},
});
