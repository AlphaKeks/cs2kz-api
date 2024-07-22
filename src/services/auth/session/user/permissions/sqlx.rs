//! Trait implementations for the [`sqlx`] crate.

use super::Permissions;

impl<DB> sqlx::Type<DB> for Permissions
where
	DB: sqlx::Database,
	u64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<u64 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<u64 as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Permissions
where
	DB: sqlx::Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0, buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		<u64 as sqlx::Encode<'q, DB>>::encode(self.0, buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		<u64 as sqlx::Encode<'q, DB>>::produces(&self.0)
	}

	fn size_hint(&self) -> usize
	{
		<u64 as sqlx::Encode<'q, DB>>::size_hint(&self.0)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Permissions
where
	DB: sqlx::Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		let bits = <u64 as sqlx::Decode<'r, DB>>::decode(value)?;
		let permissions = Permissions::new(bits);

		Ok(permissions)
	}
}
