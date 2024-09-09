use std::cmp;

use serde::{Deserialize, Serialize};

#[derive(
	Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] time::OffsetDateTime);

impl Timestamp
{
	pub fn now() -> Self
	{
		Self(time::OffsetDateTime::now_utc())
	}
}

impl From<time::OffsetDateTime> for Timestamp
{
	fn from(datetime: time::OffsetDateTime) -> Self
	{
		Self(datetime)
	}
}

impl From<Timestamp> for time::OffsetDateTime
{
	fn from(Timestamp(datetime): Timestamp) -> Self
	{
		datetime
	}
}

impl PartialEq<time::OffsetDateTime> for Timestamp
{
	fn eq(&self, other: &time::OffsetDateTime) -> bool
	{
		self.0.eq(other)
	}
}

impl PartialEq<Timestamp> for time::OffsetDateTime
{
	fn eq(&self, other: &Timestamp) -> bool
	{
		self.eq(&other.0)
	}
}

impl PartialOrd<time::OffsetDateTime> for Timestamp
{
	fn partial_cmp(&self, other: &time::OffsetDateTime) -> Option<cmp::Ordering>
	{
		self.0.partial_cmp(other)
	}
}

impl PartialOrd<Timestamp> for time::OffsetDateTime
{
	fn partial_cmp(&self, other: &Timestamp) -> Option<cmp::Ordering>
	{
		self.partial_cmp(&other.0)
	}
}
