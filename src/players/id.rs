use std::{error::Error, str::FromStr};

use serde::{Deserialize, Serialize};
use steam_id::{ParseSteamIdError, SteamId};
use utoipa::ToSchema;

use crate::users::UserId;

#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	From,
	Into,
	Serialize,
	Deserialize,
	ToSchema,
)]
#[debug("PlayerId({})", _0.as_u64())]
#[serde(transparent)]
#[schema(value_type = str, format = UInt64, example = "STEAM_1:1:161178172")]
pub struct PlayerId(SteamId);

#[derive(Debug, Display, From, Error)]
pub struct ParsePlayerIdError(ParseSteamIdError);

impl AsRef<SteamId> for PlayerId
{
	fn as_ref(&self) -> &SteamId
	{
		&self.0
	}
}

impl PartialEq<UserId> for PlayerId
{
	fn eq(&self, other: &UserId) -> bool
	{
		self.as_ref() == other.as_ref()
	}
}

impl FromStr for PlayerId
{
	type Err = ParsePlayerIdError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<SteamId>().map(Self).map_err(ParsePlayerIdError)
	}
}

impl<DB> sqlx::Type<DB> for PlayerId
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

impl<'q, DB> sqlx::Encode<'q, DB> for PlayerId
where
	DB: sqlx::Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		self.0.as_ref().encode_by_ref(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		self.0.as_ref().produces()
	}

	fn size_hint(&self) -> usize
	{
		self.0.as_ref().size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PlayerId
where
	DB: sqlx::Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		Ok(Self(u64::decode(value)?.try_into()?))
	}
}
