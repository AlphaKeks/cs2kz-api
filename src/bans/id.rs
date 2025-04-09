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
#[schema(value_type = u32)]
pub struct BanId(NonZero<u32>);

#[derive(Debug, Display, From, Error)]
pub struct ParseBanIdError(ParseIntError);

impl FromStr for BanId
{
	type Err = ParseBanIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u32>>().map(Self).map_err(ParseBanIdError)
	}
}
