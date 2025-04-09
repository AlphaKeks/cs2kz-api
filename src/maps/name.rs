use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{error::Error, str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "kz_grotto")]
pub struct MapName(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid map name: {_variant}")]
pub enum InvalidMapName
{
	#[display("must start with `kz_`")]
	MissingPrefix,

	#[display("must be 4-27 characters long")]
	InvalidLength,

	#[display("must not contain {_0:?}")]
	#[error(ignore)]
	InvalidCharacter(char),
}

impl MapName
{
	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for MapName
{
	type Err = InvalidMapName;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if !value.starts_with("kz_") {
			return Err(InvalidMapName::MissingPrefix);
		}

		let len = value.chars().try_fold(0_usize, |len, char| {
			if len == 27_usize {
				return Err(InvalidMapName::InvalidLength);
			}

			if !matches!(char, 'a'..='z' | 'A'..='Z' | '_') {
				return Err(InvalidMapName::InvalidCharacter(char));
			}

			Ok(len + 1_usize)
		})?;

		if len < 4_usize {
			return Err(InvalidMapName::InvalidLength);
		}

		Ok(Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for MapName
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct MapNameVisitor;

		impl de::Visitor<'_> for MapNameVisitor
		{
			type Value = MapName;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a KZ map name")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				value.parse().map_err(E::custom)
			}
		}

		deserializer.deserialize_string(MapNameVisitor)
	}
}

impl<DB> sqlx::Type<DB> for MapName
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

impl<'q, DB> sqlx::Encode<'q, DB> for MapName
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

impl<'r, DB> sqlx::Decode<'r, DB> for MapName
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
