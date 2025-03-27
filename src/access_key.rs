//! API access keys
//!
//! This module contains the [`AccessKey`] struct which is a shared abstraction
//! for opaque access control. It implements all the necessary traits that
//! downstream consumers may need.
//!
//! The current implementation uses [UUIDs] (version 7), but that may change in
//! the future. The following public API should be maintained even if the
//! underlying implementation changes:
//!
//! - [`AccessKey::new()`] for generating a new key
//! - [`AccessKey::invalid()`] for generating an "invalid" sentinel key
//! - [`AccessKey::is_invalid()`] for checking whether a given key is the sentinel value returned by
//!   [`AccessKey::invalid()`]
//!
//! [UUIDs]: ::uuid

use std::str::FromStr;

use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use zerocopy::TryFromBytes;

use crate::database::{DatabaseConnection, DatabaseError, DatabaseResult};

#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[schema(format = Uuid, example = "bf631097-05fa-439c-8538-e471874f03ba")]
pub struct AccessKey(Uuid);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse access key")]
pub struct ParseAccessKeyError(uuid::Error);

impl AccessKey
{
	/// Generates a new [`AccessKey`].
	#[expect(clippy::new_without_default, reason = "keys are generated randomly")]
	pub fn new() -> Self
	{
		Self(Uuid::now_v7())
	}

	/// Returns a sentinel value that is considered to be "invalid".
	pub fn invalid() -> Self
	{
		Self(Uuid::nil())
	}

	/// Checks whether `self` is the sentinel value returned by [`AccessKey::invalid()`].
	pub fn is_invalid(&self) -> bool
	{
		self.0.is_nil()
	}

	#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
	pub async fn fetch_name(
		&self,
		conn: &mut DatabaseConnection<'_, '_>,
	) -> DatabaseResult<Option<Box<str>>>
	{
		sqlx::query_scalar!("SELECT name AS `name: Box<str>` FROM AccessKeys WHERE value = ?", self)
			.fetch_optional(conn.as_raw())
			.map_err(DatabaseError::from)
			.await
	}
}

impl FromStr for AccessKey
{
	type Err = ParseAccessKeyError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value.parse::<Uuid>().map(Self).map_err(ParseAccessKeyError::from)
	}
}

impl<DB> sqlx::Type<DB> for AccessKey
where
	DB: sqlx::Database,
	[u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<[u8] as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for AccessKey
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		(&self.0.as_bytes()[..]).encode(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		(&self.0.as_bytes()[..]).produces()
	}

	fn size_hint(&self) -> usize
	{
		(&self.0.as_bytes()[..]).size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for AccessKey
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		let bytes = <&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)?;
		let uuid = uuid::Bytes::try_read_from_bytes(bytes).map_err(|err| err.map_src(Vec::from))?;

		Ok(Self(Uuid::from_bytes(uuid)))
	}
}
