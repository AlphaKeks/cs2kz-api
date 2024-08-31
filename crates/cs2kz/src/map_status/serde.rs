//! Trait implementations for the [`serde`] crate.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::MapState;

impl Serialize for MapState
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_str(serializer)
	}
}

impl<'de> Deserialize<'de> for MapState
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
			I8(i8),
			Str(Box<str>),
		}

		Helper::deserialize(deserializer).and_then(|v| match v {
			Helper::I8(value) => Self::try_from(value).map_err(de::Error::custom),
			Helper::Str(value) => value.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

impl MapState
{
	/// Serializes a [`MapState`] as an integer.
	pub fn serialize_i8<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		i8::from(*self).serialize(serializer)
	}

	/// Deserializes a [`MapState`] from an integer.
	pub fn deserialize_i8<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		i8::deserialize(deserializer)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`MapState`] as a string.
	pub fn serialize_str<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		match self {
			Self::NotGlobal => "not_global",
			Self::InTesting => "in_testing",
			Self::Global => "global",
		}
		.serialize(serializer)
	}

	/// Deserializes a [`MapState`] from a string.
	pub fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?
			.parse()
			.map_err(de::Error::custom)
	}
}
