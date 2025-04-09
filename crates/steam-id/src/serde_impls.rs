use {
	crate::SteamId,
	serde::{
		de::{self, Deserialize, Deserializer},
		ser::{Serialize, Serializer},
	},
};

impl Serialize for SteamId
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.serialize_id2(serializer)
	}
}

impl<'de> Deserialize<'de> for SteamId
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct CatchallVisitor;

		impl de::Visitor<'_> for CatchallVisitor
		{
			type Value = SteamId;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a SteamID")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				value.parse::<SteamId>().map_err(E::custom)
			}
		}

		deserializer.deserialize_any(CatchallVisitor)
	}
}

impl SteamId
{
	/// Serializes using the Steam2ID format.
	pub fn serialize_id2<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}

	/// Serializes using the SteamID64 format.
	pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_u64().serialize(serializer)
	}

	/// Serializes using a stringified version of the SteamID64 format.
	pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		format_args!("{}", self.as_u64()).serialize(serializer)
	}

	/// Deserializes using the Steam2ID format.
	pub fn deserialize_id2<'de, D>(&self, deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct Steam2IdVisitor;

		impl de::Visitor<'_> for Steam2IdVisitor
		{
			type Value = SteamId;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a Steam2ID")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				SteamId::parse_id2(value).map_err(E::custom)
			}
		}

		deserializer.deserialize_str(Steam2IdVisitor)
	}
}
