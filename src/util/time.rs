//! Extensions for [`std::time`].

use std::ops::{Deref, DerefMut};
use std::time::Duration;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Extension trait for [`std::time::Duration`].
#[sealed]
pub trait DurationExt
{
	/// One minute.
	const MINUTE: Duration = Duration::from_secs(60);

	/// One hour.
	const HOUR: Duration = Duration::from_secs(60 * 60);

	/// One day.
	const DAY: Duration = Duration::from_secs(60 * 60 * 24);

	/// One week.
	const WEEK: Duration = Duration::from_secs(60 * 60 * 24 * 7);

	/// One month (30 days).
	const MONTH: Duration = Duration::from_secs(60 * 60 * 24 * 30);

	/// One year (365 days).
	const YEAR: Duration = Duration::from_secs(60 * 60 * 24 * 365);
}

#[sealed]
impl DurationExt for Duration {}

/// A thin wrapper around [`std::time::Duration`] that ensures encoding/decoding
/// always happens in terms of seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seconds(pub Duration);

impl Deref for Seconds
{
	type Target = Duration;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl DerefMut for Seconds
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		&mut self.0
	}
}

impl From<Duration> for Seconds
{
	fn from(value: Duration) -> Self
	{
		Self(value)
	}
}

impl From<Seconds> for Duration
{
	fn from(value: Seconds) -> Self
	{
		value.0
	}
}

impl Serialize for Seconds
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_secs_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer)
			.map(Duration::from_secs_f64)
			.map(Self)
	}
}

impl<DB> sqlx::Type<DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		todo!()
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		<f64 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.as_secs_f64(), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		<f64 as sqlx::Encode<'q, DB>>::encode(self.as_secs_f64(), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		<f64 as sqlx::Encode<'q, DB>>::produces(&self.as_secs_f64())
	}

	fn size_hint(&self) -> usize
	{
		<f64 as sqlx::Encode<'q, DB>>::size_hint(&self.as_secs_f64())
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		<f64 as sqlx::Decode<'r, DB>>::decode(value)
			.map(Duration::from_secs_f64)
			.map(Self)
	}
}
