use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "Alpha's KZ")]
pub struct ServerName(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid server name: {_variant}")]
pub enum InvalidServerName
{
	#[display("may not be empty")]
	Empty,
}

impl ServerName
{
	fn validate(value: &str) -> Result<(), InvalidServerName>
	{
		if value.is_empty() {
			return Err(InvalidServerName::Empty);
		}

		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for ServerName
{
	type Err = InvalidServerName;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for ServerName
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ServerNameVisitor;

		impl de::Visitor<'_> for ServerNameVisitor
		{
			type Value = ServerName;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a KZ server name")
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
				ServerName::validate(&value)
					.map(|()| ServerName(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(ServerNameVisitor)
	}
}

impl_sqlx!(ServerName => {
	Type as str;
	Encode<'q, 'a> as &'a str = |name| name.as_str();
	Decode<'r> as String = |value| {
		ServerName::validate(&value)
			.map(|()| ServerName(value.into()))
	};
});
