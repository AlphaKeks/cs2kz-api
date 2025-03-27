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
#[schema(value_type = u32)]
pub struct RecordId(NonZero<u32>);

#[derive(Debug, Display, From, Error)]
pub struct ParseRecordIdError(ParseIntError);

impl FromStr for RecordId
{
	type Err = ParseRecordIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u32>>().map(Self).map_err(ParseRecordIdError)
	}
}
