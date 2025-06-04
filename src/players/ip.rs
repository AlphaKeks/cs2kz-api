use {
	serde::{Deserialize, Serialize},
	std::net::Ipv4Addr,
};

#[derive(Debug, Display, Clone, Copy, From, Into, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerIp(Ipv4Addr);

impl<DB> sqlx::Type<DB> for PlayerIp
where
	DB: sqlx::Database,
	Ipv4Addr: sqlx::Type<DB>,
	[u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<Ipv4Addr as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<Ipv4Addr as sqlx::Type<DB>>::compatible(ty) || <[u8] as sqlx::Type<DB>>::compatible(ty) // XXX
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for PlayerIp
where
	DB: sqlx::Database,
	Ipv4Addr: sqlx::Encode<'q, DB>,
{
	#[tracing::instrument(level = "trace", skip(buf), err)]
	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		sqlx::Encode::encode(self.0, buf)
	}

	#[tracing::instrument(level = "trace", skip(buf), err)]
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		sqlx::Encode::encode_by_ref(&self.0, buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		sqlx::Encode::produces(&self.0)
	}

	fn size_hint(&self) -> usize
	{
		sqlx::Encode::size_hint(&self.0)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PlayerIp
where
	DB: sqlx::Database,
	Ipv4Addr: sqlx::Decode<'r, DB>,
{
	#[tracing::instrument(level = "trace", skip(value), ret(level = "trace"), err(level = "debug"))]
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		<Ipv4Addr as sqlx::Decode<'r, DB>>::decode(value).map(Self)
	}
}
