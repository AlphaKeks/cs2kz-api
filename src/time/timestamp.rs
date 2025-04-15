use {
	serde::{Deserialize, Serialize},
	std::{
		cmp,
		ops,
		time::{Duration, SystemTime},
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
#[sqlx(transparent)]
pub struct Timestamp(#[serde(with = "time::serde::rfc3339")] time::OffsetDateTime);

impl Timestamp
{
	pub fn now() -> Self
	{
		Self(time::OffsetDateTime::now_utc())
	}

	/// Computes the time difference between `self` and `earlier` as a [`Duration`].
	///
	/// A return value of `Err` indicates that `self` happened before `earlier`.
	pub fn duration_since(self, earlier: Self) -> Result<Duration, Duration>
	{
		SystemTime::from(self)
			.duration_since(earlier.into())
			.map_err(|err| err.duration())
	}

	/// Computes the time difference between "now" and `self` as a [`Duration`].
	///
	/// A return value of `Err` indicates that `self` is in the future.
	pub fn elapsed(self) -> Result<Duration, Duration>
	{
		Self::now().duration_since(self)
	}
}

impl From<SystemTime> for Timestamp
{
	fn from(system_time: SystemTime) -> Self
	{
		Self(system_time.into())
	}
}

impl From<Timestamp> for SystemTime
{
	fn from(timestamp: Timestamp) -> Self
	{
		timestamp.0.into()
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

impl ops::Add<std::time::Duration> for Timestamp
{
	type Output = Timestamp;

	fn add(self, rhs: std::time::Duration) -> Self::Output
	{
		Timestamp(self.0 + rhs)
	}
}

impl ops::Add<Timestamp> for std::time::Duration
{
	type Output = Timestamp;

	fn add(self, rhs: Timestamp) -> Self::Output
	{
		Timestamp(rhs.0 + self)
	}
}

impl ops::Add<time::Duration> for Timestamp
{
	type Output = Timestamp;

	fn add(self, rhs: time::Duration) -> Self::Output
	{
		Timestamp(self.0 + rhs)
	}
}

impl ops::Add<Timestamp> for time::Duration
{
	type Output = Timestamp;

	fn add(self, rhs: Timestamp) -> Self::Output
	{
		Timestamp(rhs.0 + self)
	}
}
