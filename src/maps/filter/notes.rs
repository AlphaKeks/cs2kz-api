use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Default, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str)]
pub struct FilterNotes(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid filter notes: {_variant}")]
pub enum InvalidFilterNotes {}

impl FilterNotes
{
	fn validate(_value: &str) -> Result<(), InvalidFilterNotes>
	{
		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for FilterNotes
{
	type Err = InvalidFilterNotes;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for FilterNotes
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct FilterNotesVisitor;

		impl de::Visitor<'_> for FilterNotesVisitor
		{
			type Value = FilterNotes;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("KZ filter notes")
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
				FilterNotes::validate(&value)
					.map(|()| FilterNotes(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(FilterNotesVisitor)
	}
}

impl_sqlx!(FilterNotes => {
	Type as str;
	Encode<'q, 'a> as &'a str = |notes| notes.as_str();
	Decode<'r> as String = |value| {
		FilterNotes::validate(&value)
			.map(|()| FilterNotes(value.into()))
	};
});
