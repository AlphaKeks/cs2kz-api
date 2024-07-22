//! API keys for CS2 servers.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// An API key for CS2 servers.
#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct ApiKey(Uuid);

impl ApiKey
{
	/// Generates a new random key.
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}
}

impl fmt::Debug for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_tuple("ApiKey").field(&"*****").finish()
	}
}

impl fmt::Display for ApiKey
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "{}", self.0.as_hyphenated())
	}
}

impl<DB> sqlx::Type<DB> for ApiKey
where
	DB: sqlx::Database,
	Hyphenated: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<Hyphenated as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<Hyphenated as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for ApiKey
where
	DB: sqlx::Database,
	Hyphenated: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		<Hyphenated as sqlx::Encode<'q, DB>>::encode_by_ref(self.0.as_hyphenated(), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		<Hyphenated as sqlx::Encode<'q, DB>>::encode(*self.0.as_hyphenated(), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		<Hyphenated as sqlx::Encode<'q, DB>>::produces(self.0.as_hyphenated())
	}

	fn size_hint(&self) -> usize
	{
		<Hyphenated as sqlx::Encode<'q, DB>>::size_hint(self.0.as_hyphenated())
	}
}
