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
#[schema(value_type = u64)]
pub struct ServerSessionId(NonZero<u64>);

#[derive(Debug, Display, From, Error)]
pub struct ParseServerSessionIdError(ParseIntError);

impl ServerSessionId
{
	pub fn as_u64(self) -> u64
	{
		self.0.get()
	}
}

impl FromStr for ServerSessionId
{
	type Err = ParseServerSessionIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u64>>().map(Self).map_err(ParseServerSessionIdError)
	}
}
