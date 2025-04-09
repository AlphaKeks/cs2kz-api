use {
	crate::{servers::ServerId, users::UserId},
	serde::{Deserialize, Serialize},
	std::{error::Error, num::NonZero},
	steam_id::SteamId,
	utoipa::ToSchema,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, From, ToSchema)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BannedBy
{
	Server
	{
		server_id: ServerId
	},
	Admin
	{
		user_id: UserId
	},
}

#[derive(Debug, Display, Error)]
#[display("out of range value")]
pub struct TryFromU64Error(#[error(ignore)] ());

impl BannedBy
{
	pub fn as_u64(self) -> u64
	{
		match self {
			Self::Server { server_id } => server_id.as_u16().into(),
			Self::Admin { user_id } => user_id.as_ref().as_u64(),
		}
	}
}

impl TryFrom<u64> for BannedBy
{
	type Error = TryFromU64Error;

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		if let Some(server_id) = try { NonZero::new(u16::try_from(value).ok()?)? } {
			return Ok(Self::Server { server_id: server_id.into() });
		}

		if let Ok(steam_id) = SteamId::from_u64(value) {
			return Ok(Self::Admin { user_id: steam_id.into() });
		}

		Err(TryFromU64Error(()))
	}
}

impl<DB> sqlx::Type<DB> for BannedBy
where
	DB: sqlx::Database,
	u64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		u64::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		u64::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for BannedBy
where
	DB: sqlx::Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		self.as_u64().encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		self.as_u64().produces()
	}

	fn size_hint(&self) -> usize
	{
		self.as_u64().size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for BannedBy
where
	DB: sqlx::Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		Ok(u64::decode(value)?.try_into()?)
	}
}
