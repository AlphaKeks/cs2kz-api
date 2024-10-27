use std::num::{NonZero, ParseIntError};
use std::str::FromStr;

/// An ID uniquely identifying a CS2 server.
#[derive(
	Debug, Clone, Copy, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(value_type = u16)]
pub struct ServerID(NonZero<u16>);

impl FromStr for ServerID {
	type Err = ParseIntError;

	fn from_str(str: &str) -> Result<Self, Self::Err> {
		str.parse::<NonZero<u16>>().map(Self)
	}
}
