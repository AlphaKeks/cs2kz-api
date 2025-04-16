use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Default, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "Main")]
pub struct CourseDescription(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid course description: {_variant}")]
pub enum InvalidCourseDescription {}

impl CourseDescription
{
	fn validate(_value: &str) -> Result<(), InvalidCourseDescription>
	{
		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for CourseDescription
{
	type Err = InvalidCourseDescription;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for CourseDescription
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct CourseDescriptionVisitor;

		impl de::Visitor<'_> for CourseDescriptionVisitor
		{
			type Value = CourseDescription;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a KZ course description")
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
				CourseDescription::validate(&value)
					.map(|()| CourseDescription(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(CourseDescriptionVisitor)
	}
}

impl_sqlx!(CourseDescription => {
	Type as str;
	Encode<'q, 'a> as &'a str = |description| description.as_str();
	Decode<'r> as String = |value| {
		CourseDescription::validate(&value)
			.map(|()| CourseDescription(value.into()))
	};
});
