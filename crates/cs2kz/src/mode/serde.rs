//! Trait implementations for the [`serde`] crate.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::Mode;

impl Serialize for Mode
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_str(serializer)
	}
}

impl<'de> Deserialize<'de> for Mode
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[allow(clippy::missing_docs_in_private_items)]
		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		enum Helper
		{
			U8(u8),
			Str(Box<str>),
		}

		Helper::deserialize(deserializer).and_then(|v| match v {
			Helper::U8(value) => Self::try_from(value).map_err(de::Error::custom),
			Helper::Str(value) => value.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

impl Mode
{
	/// Serializes a [`Mode`] as an integer.
	pub fn serialize_u8<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		u8::from(*self).serialize(serializer)
	}

	/// Deserializes a [`Mode`] from an integer.
	pub fn deserialize_u8<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		u8::deserialize(deserializer)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`Mode`] as a string.
	pub fn serialize_str<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			Self::Vanilla => "vanilla",
			Self::Classic => "classic",
		}
		.serialize(serializer)
	}

	/// Deserializes a [`Mode`] from a string.
	pub fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?
			.parse()
			.map_err(de::Error::custom)
	}
}
