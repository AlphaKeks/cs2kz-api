use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "AlphaKeks")]
pub struct PlayerName(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid player name: {_variant}")]
pub enum InvalidPlayerName
{
	#[display("may not be empty")]
	Empty,
}

impl PlayerName
{
	fn validate(value: &str) -> Result<(), InvalidPlayerName>
	{
		if value.is_empty() {
			return Err(InvalidPlayerName::Empty);
		}

		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for PlayerName
{
	type Err = InvalidPlayerName;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for PlayerName
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct PlayerNameVisitor;

		impl de::Visitor<'_> for PlayerNameVisitor
		{
			type Value = PlayerName;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a player name")
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
				PlayerName::validate(&value)
					.map(|()| PlayerName(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(PlayerNameVisitor)
	}
}

impl_sqlx!(PlayerName => {
	Type as str;
	Encode<'q, 'a> as &'a str = |name| name.as_str();
	Decode<'r> as String = |value| {
		PlayerName::validate(&value)
			.map(|()| PlayerName(value.into()))
	};
});
