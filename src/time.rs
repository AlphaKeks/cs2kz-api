//! Utility types for dealing with time.

use std::time::Duration;

use derive_more::{Debug, Deref, DerefMut, Display, From, Into};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::MySql;
use utoipa::ToSchema;

/// Wrapper around [`std::time::Duration`], which takes care of encoding / decoding as seconds.
#[derive(Debug, Display, Deref, DerefMut, From, Into, ToSchema)]
#[display("{:.3}", self.as_secs_f64())]
#[schema(value_type = f64)]
pub struct Seconds(pub Duration);

impl sqlx::Type<MySql> for Seconds {
	fn type_info() -> <MySql as sqlx::Database>::TypeInfo {
		f64::type_info()
	}
}

impl<'q> sqlx::Encode<'q, MySql> for Seconds {
	fn encode_by_ref(&self, buf: &mut <MySql as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
		self.as_secs_f64().encode_by_ref(buf)
	}
}

impl<'q> sqlx::Decode<'q, MySql> for Seconds {
	fn decode(value: <MySql as HasValueRef<'q>>::ValueRef) -> Result<Self, BoxDynError> {
		f64::decode(value).map(Duration::from_secs_f64).map(Self)
	}
}

impl Serialize for Seconds {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_secs_f64().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Seconds {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		f64::deserialize(deserializer)
			.map(Duration::from_secs_f64)
			.map(Self)
	}
}

/// Wrapper around [`std::time::Duration`], which takes care of encoding / decoding as weeks.
#[derive(Debug, Display, Deref, DerefMut, From, Into, ToSchema)]
#[display("{}", self.as_weeks())]
#[schema(value_type = u16)]
pub struct Weeks(pub Duration);

impl Weeks {
	/// Returns the amount of weeks.
	pub fn as_weeks(&self) -> u16 {
		u16::try_from(self.0.as_secs() / 60 / 60 / 24 / 7).expect("invalid amount of seconds")
	}
}

impl Serialize for Weeks {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.as_weeks().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Weeks {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		u16::deserialize(deserializer)
			.map(|weeks| (weeks as u64) * 7 * 24 * 60 * 60)
			.map(Duration::from_secs)
			.map(Self)
	}
}
