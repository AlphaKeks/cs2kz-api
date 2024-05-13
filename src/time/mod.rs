//! Helpers around [`std::time`].

use std::error::Error as StdError;
use std::time::Duration;

use derive_more::{Display, From, Into};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;

/// A wrapper around [`std::time::Duration`] that will always encode/decode as seconds.
#[derive(Debug, Display, Clone, Copy, From, Into)]
#[display("{_0:?}")]
pub struct Seconds(Duration);

impl Serialize for Seconds {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.as_secs().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		u64::deserialize(deserializer)
			.map(Duration::from_secs)
			.map(From::from)
	}
}

impl<DB> sqlx::Type<DB> for Seconds
where
	DB: sqlx::Database,
	u64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<u64 as sqlx::Type<DB>>::type_info()
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Seconds
where
	DB: sqlx::Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
		<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0.as_secs(), buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Seconds
where
	DB: sqlx::Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as HasValueRef<'r>>::ValueRef,
	) -> Result<Self, Box<dyn StdError + 'static + Send + Sync>> {
		<u64 as sqlx::Decode<'r, DB>>::decode(value)
			.map(Duration::from_secs)
			.map(From::from)
	}
}
