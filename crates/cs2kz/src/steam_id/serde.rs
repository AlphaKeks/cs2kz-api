//! Trait implementations for the [`serde`] crate.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use super::SteamID;

impl Serialize for SteamID
{
	/// Serializes a [`SteamID]` using the standard formatting.
	///
	/// If you want to use a different format, use the `#[serde(serialize_with = "…")]`
	/// attribute with one of the inherent `serialize_*` methods on [`SteamID`].
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_standard(serializer)
	}
}

impl<'de> Deserialize<'de> for SteamID
{
	/// Deserializes a [`SteamID`] trying to catch as many formats as possible.
	///
	/// If you expect a specific format, use the `#[serde(deserialize_with = "…")]` attribute
	/// with one of the inherent `deserialize_*` methods on [`SteamID`].
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[allow(clippy::missing_docs_in_private_items)]
		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		enum Helper
		{
			U32(u32),
			U64(u64),
			Str(Box<str>),
		}

		Helper::deserialize(deserializer).and_then(|v| match v {
			Helper::U32(value) => Self::try_from(value).map_err(de::Error::custom),
			Helper::U64(value) => Self::try_from(value).map_err(de::Error::custom),
			Helper::Str(value) => value.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

impl SteamID
{
	/// Serializes a [`SteamID`] as a 64-bit integer.
	pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		<u64 as Serialize>::serialize(&**self, serializer)
	}

	/// Deserializes a 64-bit integer into a [`SteamID`].
	pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		u64::deserialize(deserializer)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`SteamID`] as a stringified 64-bit integer.
	pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_u64().to_string().serialize(serializer)
	}

	/// Deserializes a stringified 64-bit integer into a [`SteamID`].
	pub fn deserialize_u64_stringified<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?
			.parse::<u64>()
			.map_err(de::Error::custom)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`SteamID`] as a 32-bit integer.
	pub fn serialize_u32<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_u32().serialize(serializer)
	}

	/// Deserializes a 32-bit integer into a [`SteamID`].
	pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		u32::deserialize(deserializer)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`SteamID`] as a stringified 32-bit integer.
	pub fn serialize_u32_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_u32().to_string().serialize(serializer)
	}

	/// Deserializes a stringified 32-bit integer into a [`SteamID`].
	pub fn deserialize_u32_stringified<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?
			.parse::<u32>()
			.map_err(de::Error::custom)?
			.try_into()
			.map_err(de::Error::custom)
	}

	/// Serializes a [`SteamID`] using the standard format.
	pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}

	/// Deserializes a [`SteamID`] using the standard format.
	pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Self::parse_standard(&String::deserialize(deserializer)?).map_err(de::Error::custom)
	}

	/// Serializes a [`SteamID`] using the community format.
	pub fn serialize_community<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		format_args!("{self:#}").serialize(serializer)
	}

	/// Deserializes a [`SteamID`] using the community format.
	pub fn deserialize_community<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Self::parse_community(&String::deserialize(deserializer)?).map_err(de::Error::custom)
	}
}
