//! Trait implementations for the [`sqlx`] crate.

use crate::Styles;

impl<DB> sqlx::Type<DB> for Styles
where
	DB: sqlx::Database,
	u32: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<u32 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<u32 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Styles
where
	DB: sqlx::Database,
	u32: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		<u32 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0, buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		<u32 as sqlx::Encode<'q, DB>>::encode(self.0, buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
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
	DB: sqlx::Database,
	u32: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		let bits = <u32 as sqlx::Decode<'r, DB>>::decode(value)?;
		let styles = Styles::new(bits);

		assert_eq!(styles.bits(), bits, "database should not contain invalid style bits");

		Ok(styles)
	}
}
