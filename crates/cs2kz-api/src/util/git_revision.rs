use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct GitRevision(Box<str>);

#[derive(Debug, Error)]
pub enum InvalidGitRevision
{
	#[error("git revision must be 40 characters long")]
	InvalidLength,

	#[error("git revision must be valid ASCII")]
	InvalidAscii,
}

impl FromStr for GitRevision
{
	type Err = InvalidGitRevision;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.len() != 40 {
			return Err(InvalidGitRevision::InvalidLength);
		}

		if !value.is_ascii() {
			return Err(InvalidGitRevision::InvalidAscii);
		}

		Ok(Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for GitRevision
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		let s = String::deserialize(deserializer)?;

		if s.len() != 40 {
			return Err(de::Error::invalid_length(s.len(), &"40"));
		}

		if !s.is_ascii() {
			return Err(de::Error::custom("git revision must be valid ASCII"));
		}

		Ok(Self(s.into_boxed_str()))
	}
}
