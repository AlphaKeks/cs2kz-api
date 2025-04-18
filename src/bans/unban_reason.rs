use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str)]
pub struct UnbanReason(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid unban reason: {_variant}")]
pub enum InvalidUnbanReason
{
	#[display("may not be empty")]
	Empty,
}

impl UnbanReason
{
	fn validate(value: &str) -> Result<(), InvalidUnbanReason>
	{
		if value.is_empty() {
			return Err(InvalidUnbanReason::Empty);
		}

		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for UnbanReason
{
	type Err = InvalidUnbanReason;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for UnbanReason
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct UnbanReasonVisitor;

		impl de::Visitor<'_> for UnbanReasonVisitor
		{
			type Value = UnbanReason;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("an unban reason")
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
				UnbanReason::validate(&value)
					.map(|()| UnbanReason(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(UnbanReasonVisitor)
	}
}

impl_sqlx!(UnbanReason => {
	Type as str;
	Encode<'q, 'a> as &'a str = |reason| reason.as_str();
	Decode<'r> as String = |value| {
		UnbanReason::validate(&value)
			.map(|()| UnbanReason(value.into()))
	};
});
