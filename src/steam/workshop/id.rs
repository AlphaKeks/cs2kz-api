use {
	serde::{Deserialize, Serialize},
	std::{num::ParseIntError, str::FromStr},
	utoipa::ToSchema,
};

#[derive(
	Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct WorkshopId(u32);

#[derive(Debug, Display, From, Error)]
#[display("failed to parse workshop ID: {_0}")]
pub struct ParseWorkshopIdError(ParseIntError);

impl FromStr for WorkshopId
{
	type Err = ParseWorkshopIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<u32>().map(Self).map_err(ParseWorkshopIdError)
	}
}

impl_rand!(WorkshopId => |rng| WorkshopId(rng.random::<u32>()));
