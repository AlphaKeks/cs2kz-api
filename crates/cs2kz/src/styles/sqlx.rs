//! Trait implementations for the [`sqlx`] crate.

use sqlx::encode::IsNull;
use sqlx::Database;

use super::Styles;

impl<DB> sqlx::Type<DB> for Styles
where
	DB: Database,
	u32: sqlx::Type<DB>,
{
	fn type_info() -> <DB as Database>::TypeInfo
	{
		<u32 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as Database>::TypeInfo) -> bool
	{
		<u32 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Styles
where
	DB: Database,
	u32: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<u32 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0, buf)
	}

	fn encode(
		self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<u32 as sqlx::Encode<'q, DB>>::encode(self.0, buf)
	}

	fn produces(&self) -> Option<<DB as Database>::TypeInfo>
	{
		<u32 as sqlx::Encode<'q, DB>>::produces(&self.0)
	}

	fn size_hint(&self) -> usize
	{
		<u32 as sqlx::Encode<'q, DB>>::size_hint(&self.0)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Styles
where
	DB: Database,
	u32: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		<u32 as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map_err(Into::into)
	}
}
