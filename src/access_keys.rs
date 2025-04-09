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
//! - [`AccessKey::new()`] - for generating a new key
//! - [`AccessKey::INVALID`] - a sentinel value representing an "invalid" key
//! - [`AccessKey::is_invalid()`] - for checking whether a given key is
//!   [`AccessKey::INVALID`]
//!
//! [UUIDs]: ::uuid

use {
	crate::database::{self, DatabaseError, DatabaseResult},
	futures_util::TryFutureExt,
	serde::{Deserialize, Serialize},
	std::str::FromStr,
	utoipa::ToSchema,
	uuid::Uuid,
	zerocopy::TryFromBytes,
};

/// An API access key
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::access_keys
#[repr(transparent)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[schema(format = Uuid, example = "bf631097-05fa-439c-8538-e471874f03ba")]
pub struct AccessKey(Uuid);

/// Error for parsing strings into [`AccessKey`]s
#[derive(Debug, Display, Error, From)]
#[display("failed to parse access key")]
pub struct ParseAccessKeyError(uuid::Error);

impl AccessKey
{
	/// An "invalid" [`AccessKey`]
	///
	/// If [`.is_invalid()`] is invoked on this value, it will return `true`.
	///
	/// [`.is_invalid()`]: AccessKey::is_invalid()
	pub const INVALID: Self = Self(Uuid::nil());

	/// Generates a new (random) [`AccessKey`].
	#[expect(clippy::new_without_default, reason = "keys are generated randomly")]
	pub fn new() -> Self
	{
		Self(Uuid::now_v7())
	}

	/// Returns whether `self` is an [invalid] [`AccessKey`].
	///
	/// [invalid]: AccessKey::INVALID
	pub const fn is_invalid(&self) -> bool
	{
		self.0.is_nil()
	}

	/// Returns the raw bytes that the access key consists of.
	pub const fn as_bytes(&self) -> &[u8]
	{
		self.0.as_bytes().as_slice()
	}

	/// Fetches the name[^table] of this access key.
	///
	/// [^table]: see the `AccessKeys` database table
	#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
	pub async fn fetch_name(
		&self,
		db_conn: &mut database::Connection<'_, '_>,
	) -> DatabaseResult<Option<Box<str>>>
	{
		sqlx::query_scalar!("SELECT name AS `name: Box<str>` FROM AccessKeys WHERE value = ?", self)
			.fetch_optional(db_conn.raw_mut())
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

impl_sqlx!(AccessKey => {
	Type as [u8];
	Encode<'q, 'a> as &'a [u8] = |access_key| access_key.as_bytes();
	Decode<'r> as &'r [u8] = |bytes| {
		uuid::Bytes::try_read_from_bytes(bytes)
			.map(|bytes| Self(Uuid::from_bytes(bytes)))
			.map_err(|err| err.map_src(Vec::from))
	};
});
