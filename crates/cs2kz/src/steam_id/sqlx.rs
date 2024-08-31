//! Trait implementations for the [`sqlx`] crate.

use std::borrow::Borrow;

use sqlx::encode::IsNull;
use sqlx::Database;

use super::SteamID;

impl<DB> sqlx::Type<DB> for SteamID
where
	DB: Database,
	u64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as Database>::TypeInfo
	{
		<u64 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as Database>::TypeInfo) -> bool
	{
		<u64 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for SteamID
where
	DB: Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(self.borrow(), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<u64 as sqlx::Encode<'q, DB>>::encode(*self, buf)
	}

	fn produces(&self) -> Option<<DB as Database>::TypeInfo>
	{
		<u64 as sqlx::Encode<'q, DB>>::produces(self.borrow())
	}

	fn size_hint(&self) -> usize
	{
		<u64 as sqlx::Encode<'q, DB>>::size_hint(self.borrow())
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for SteamID
where
	DB: Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		<u64 as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map_err(Into::into)
	}
}
