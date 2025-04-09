use {
	serde::Serialize,
	std::{error::Error, str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "AlphaKeks")]
pub struct Username(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid username: {_variant}")]
pub enum InvalidUsername
{
	#[display("may not be empty")]
	Empty,
}

impl Username
{
	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for Username
{
	type Err = InvalidUsername;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.is_empty() {
			return Err(InvalidUsername::Empty);
		}

		Ok(Self(value.into()))
	}
}

impl<DB> sqlx::Type<DB> for Username
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

impl<'q, DB> sqlx::Encode<'q, DB> for Username
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

impl<'r, DB> sqlx::Decode<'r, DB> for Username
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
