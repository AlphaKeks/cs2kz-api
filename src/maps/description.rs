use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Default, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "Alpha's KZ")]
pub struct MapDescription(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid map description: {_variant}")]
pub enum InvalidMapDescription {}

impl MapDescription
{
	fn validate(_value: &str) -> Result<(), InvalidMapDescription>
	{
		Ok(())
	}

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
		Self::validate(value).map(|()| Self(value.into()))
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

			fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				MapDescription::validate(&value)
					.map(|()| MapDescription(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(MapDescriptionVisitor)
	}
}

impl_sqlx!(MapDescription => {
	Type as str;
	Encode<'q, 'a> as &'a str = |description| description.as_str();
	Decode<'r> as String = |value| {
		MapDescription::validate(&value)
			.map(|()| MapDescription(value.into()))
	};
});
