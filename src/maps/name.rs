use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
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
	fn validate(value: &str) -> Result<(), InvalidMapName>
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

		Ok(())
	}

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
		Self::validate(value).map(|()| Self(value.into()))
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

			fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				MapName::validate(&value)
					.map(|()| MapName(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(MapNameVisitor)
	}
}

impl_sqlx!(MapName => {
	Type as str;
	Encode<'q, 'a> as &'a str = |name| name.as_str();
	Decode<'r> as String = |value| {
		MapName::validate(&value)
			.map(|()| MapName(value.into()))
	};
});
