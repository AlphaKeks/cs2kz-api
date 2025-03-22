use std::{error::Error, time::Duration};

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use utoipa::ToSchema;

/// A wrapper around [`Duration`] that ensures encoding/decoding always happens
/// in terms of seconds
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, From, Into, ToSchema)]
#[schema(value_type = f64, description = "A duration in seconds")]
pub struct Seconds(pub Duration);

impl Seconds
{
	pub const fn as_f64(self) -> f64
	{
		self.0.as_secs_f64()
	}
}

impl From<f64> for Seconds
{
	fn from(value: f64) -> Self
	{
		Self(Duration::from_secs_f64(value))
	}
}

impl Serialize for Seconds
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer).map(Self::from)
	}
}

impl<DB> sqlx::Type<DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		f64::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		f64::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		self.as_f64().encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		self.as_f64().produces()
	}

	fn size_hint(&self) -> usize
	{
		self.as_f64().size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Seconds
where
	DB: sqlx::Database,
	f64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		f64::decode(value).map(Self::from)
	}
}
