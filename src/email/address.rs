use std::{error::Error, str::FromStr, sync::Arc};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(value_type = str, format = Email)]
pub struct EmailAddress(Arc<lettre::Address>);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse email address: {_0}")]
pub struct ParseEmailAddressError(lettre::address::AddressError);

impl EmailAddress
{
	pub fn as_str(&self) -> &str
	{
		lettre::Address::as_ref(&self.0)
	}
}

impl FromStr for EmailAddress
{
	type Err = ParseEmailAddressError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value
			.parse::<lettre::Address>()
			.map(Arc::new)
			.map(Self)
			.map_err(ParseEmailAddressError)
	}
}

impl<DB> sqlx::Type<DB> for EmailAddress
where
	DB: sqlx::Database,
	str: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		str::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		str::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for EmailAddress
where
	DB: sqlx::Database,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		self.as_str().encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		self.as_str().produces()
	}

	fn size_hint(&self) -> usize
	{
		self.as_str().size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for EmailAddress
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		Ok(<&str>::decode(value)?.parse()?)
	}
}
