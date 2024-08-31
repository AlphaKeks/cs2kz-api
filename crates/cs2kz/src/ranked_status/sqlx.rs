//! Trait implementations for the [`sqlx`] crate.

use sqlx::encode::IsNull;
use sqlx::Database;

use super::RankedStatus;

impl<DB> sqlx::Type<DB> for RankedStatus
where
	DB: Database,
	i8: sqlx::Type<DB>,
{
	fn type_info() -> <DB as Database>::TypeInfo
	{
		<i8 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as Database>::TypeInfo) -> bool
	{
		<i8 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for RankedStatus
where
	DB: Database,
	i8: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<i8 as sqlx::Encode<'q, DB>>::encode_by_ref(&i8::from(*self), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as Database>::ArgumentBuffer<'q>,
	) -> Result<IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		<i8 as sqlx::Encode<'q, DB>>::encode_by_ref(&i8::from(self), buf)
	}

	fn produces(&self) -> Option<<DB as Database>::TypeInfo>
	{
		<i8 as sqlx::Encode<'q, DB>>::produces(&i8::from(*self))
	}

	fn size_hint(&self) -> usize
	{
		<i8 as sqlx::Encode<'q, DB>>::size_hint(&i8::from(*self))
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for RankedStatus
where
	DB: Database,
	i8: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		<i8 as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map_err(Into::into)
	}
}
