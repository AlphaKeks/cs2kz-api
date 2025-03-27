use std::{error::Error, str::FromStr, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, de};
use utoipa::ToSchema;

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "Alpha's KZ")]
pub struct MapDescription(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid map description: {_variant}")]
pub enum InvalidMapDescription
{
	#[display("may not be empty")]
	Empty,
}

impl MapDescription
{
	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for MapDescription
{
	type Err = InvalidMapDescription;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.is_empty() {
			return Err(InvalidMapDescription::Empty);
		}

		Ok(Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for MapDescription
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct MapDescriptionVisitor;

		impl de::Visitor<'_> for MapDescriptionVisitor
		{
			type Value = MapDescription;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a KZ map description")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				value.parse().map_err(E::custom)
			}
		}

		deserializer.deserialize_string(MapDescriptionVisitor)
	}
}

impl<DB> sqlx::Type<DB> for MapDescription
where
	DB: sqlx::Database,
	str: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		str::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		str::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for MapDescription
where
	DB: sqlx::Database,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		self.as_str().encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		self.as_str().produces()
	}

	fn size_hint(&self) -> usize
	{
		self.as_str().size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for MapDescription
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		Ok(<&str>::decode(value)?.parse()?)
	}
}
