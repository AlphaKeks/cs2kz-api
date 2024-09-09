use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::util::GitRevision;

make_id! {
	/// An ID uniquely identifying a plugin version.
	pub struct PluginVersionID(u16);
}

/// A plugin version name.
///
/// Versions follow SemVer, e.g. "1.0.2".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginVersionName(semver::Version);

impl FromStr for PluginVersionName
{
	type Err = <semver::Version as FromStr>::Err;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<semver::Version>().map(Self)
	}
}

sql_type!(PluginVersionName as String => {
	encode_by_ref: |self| &self.0.to_string(),
	encode: |self| self.0.to_string(),
	decode: |value| {
		value.parse::<semver::Version>()
			.map(Self)
			.map_err(Into::into)
	},
});

/// An identifier for a plugin version.
#[derive(Debug, Clone)]
pub enum PluginVersionIdentifier
{
	/// A version ID.
	ID(PluginVersionID),

	/// A SemVer version.
	Name(PluginVersionName),

	/// A git revision.
	Revision(GitRevision),
}

impl<'de> Deserialize<'de> for PluginVersionIdentifier
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		enum Helper
		{
			Int(u16),
			Str(String),
		}

		Helper::deserialize(deserializer).and_then(|v| match v {
			Helper::Int(int) => TryFrom::try_from(int)
				.map(PluginVersionID)
				.map(Self::ID)
				.map_err(de::Error::custom),

			Helper::Str(str) => {
				if let Ok(id) = str.parse::<PluginVersionID>() {
					Ok(Self::ID(id))
				} else if let Ok(name) = str.parse::<PluginVersionName>() {
					Ok(Self::Name(name))
				} else if let Ok(revision) = str.parse::<GitRevision>() {
					Ok(Self::Revision(revision))
				} else {
					Err(de::Error::custom("unrecognized plugin version"))
				}
			}
		})
	}
}
