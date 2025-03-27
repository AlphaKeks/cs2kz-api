use std::{error::Error, str::FromStr, sync::Arc};

use serde::{Deserialize, Deserializer, Serialize, de};
use utoipa::ToSchema;

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
		if value.is_empty() {
			return Err(InvalidCourseName::Empty);
		}

		if value.parse::<u16>().is_ok() {
			return Err(InvalidCourseName::Integer);
		}

		Ok(Self(value.into()))
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
		}

		deserializer.deserialize_string(CourseNameVisitor)
	}
}

impl<DB> sqlx::Type<DB> for CourseName
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

impl<'q, DB> sqlx::Encode<'q, DB> for CourseName
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

impl<'r, DB> sqlx::Decode<'r, DB> for CourseName
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
