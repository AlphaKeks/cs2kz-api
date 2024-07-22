//! Reasons for which players can get unbanned.

use std::convert::Infallible;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Reasons for which players can get unbanned.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, rename_all = "snake_case")]
pub enum UnbanReason
{
	/// The ban was a false ban.
	FalseBan,

	/// Some other reason.
	Other(String),
}

impl UnbanReason
{
	/// Returns a string representation of this [`UnbanReason`].
	pub fn as_str(&self) -> &str
	{
		match self {
			Self::FalseBan => "false_ban",
			Self::Other(other) => other.as_str(),
		}
	}
}

impl FromStr for UnbanReason
{
	type Err = Infallible;

	fn from_str(s: &str) -> Result<Self, Self::Err>
	{
		match s {
			"false_ban" => Ok(Self::FalseBan),
			other => Ok(Self::Other(other.to_owned())),
		}
	}
}

impl<DB> sqlx::Type<DB> for UnbanReason
where
	DB: sqlx::Database,
	String: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<String as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<String as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for UnbanReason
where
	DB: sqlx::Database,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		self.as_str().encode_by_ref(buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		self.as_str().encode(buf)
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

impl<'r, DB> sqlx::Decode<'r, DB> for UnbanReason
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		<&'r str as sqlx::Decode<'r, DB>>::decode(value)?
			.parse()
			.map_err(Into::into)
	}
}
