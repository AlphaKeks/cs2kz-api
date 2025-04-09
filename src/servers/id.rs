use {
	serde::{Deserialize, Serialize},
	std::{
		num::{NonZero, ParseIntError},
		str::FromStr,
	},
	utoipa::ToSchema,
};

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
#[schema(value_type = u16)]
pub struct ServerId(NonZero<u16>);

#[derive(Debug, Display, From, Error)]
pub struct ParseServerIdError(ParseIntError);

impl ServerId
{
	pub fn as_u16(self) -> u16
	{
		self.0.get()
	}
}

impl FromStr for ServerId
{
	type Err = ParseServerIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u16>>().map(Self).map_err(ParseServerIdError)
	}
}
