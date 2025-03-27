use std::{
	num::{NonZero, ParseIntError},
	str::FromStr,
};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	From,
	Into,
	Serialize,
	Deserialize,
	sqlx::Type,
	ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(value_type = u16)]
pub struct CourseId(NonZero<u16>);

#[derive(Debug, Display, From, Error)]
pub struct ParseCourseIdError(ParseIntError);

impl FromStr for CourseId
{
	type Err = ParseCourseIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u16>>().map(Self).map_err(ParseCourseIdError)
	}
}
