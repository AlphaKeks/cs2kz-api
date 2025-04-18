use {
	serde::{Deserialize, Deserializer, Serialize, de},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "Main")]
pub struct CourseName(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid course name: {_variant}")]
pub enum InvalidCourseName
{
	#[display("may not be empty")]
	Empty,

	#[display("may not be an integer")]
	Integer,
}

impl CourseName
{
	fn validate(value: &str) -> Result<(), InvalidCourseName>
	{
		if value.is_empty() {
			return Err(InvalidCourseName::Empty);
		}

		if value.parse::<u16>().is_ok() {
			return Err(InvalidCourseName::Integer);
		}

		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for CourseName
{
	type Err = InvalidCourseName;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl<'de> Deserialize<'de> for CourseName
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct CourseNameVisitor;

		impl de::Visitor<'_> for CourseNameVisitor
		{
			type Value = CourseName;

			fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
			{
				fmt.write_str("a KZ course name")
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
				CourseName::validate(&value)
					.map(|()| CourseName(value.into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_string(CourseNameVisitor)
	}
}

impl_sqlx!(CourseName => {
	Type as str;
	Encode<'q, 'a> as &'a str = |name| name.as_str();
	Decode<'r> as String = |value| {
		CourseName::validate(&value)
			.map(|()| CourseName(value.into()))
	};
});
