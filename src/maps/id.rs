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
pub struct MapId(NonZero<u16>);

#[derive(Debug, Display, From, Error)]
pub struct ParseMapIdError(ParseIntError);

impl FromStr for MapId
{
	type Err = ParseMapIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<NonZero<u16>>().map(Self).map_err(ParseMapIdError)
	}
}
