//! Session IDs.

use std::str::FromStr;

use derive_more::{Debug, Display, Into};
use sqlx::database::{HasArguments, HasValueRef};
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::Database;
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// A session ID.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Into)]
#[debug("*****")]
#[display("{_0}")]
pub struct SessionID(Uuid);

impl SessionID {
	/// Generate a new [`SessionID`].
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl FromStr for SessionID {
	type Err = uuid::Error;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.parse::<Uuid>().map(Self)
	}
}

impl<DB> sqlx::Type<DB> for SessionID
where
	DB: Database,
	Hyphenated: sqlx::Type<DB>,
{
	fn type_info() -> <DB as Database>::TypeInfo {
		<Hyphenated as sqlx::Type<DB>>::type_info()
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for SessionID
where
	DB: Database,
	Hyphenated: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(&self, buf: &mut <DB as HasArguments<'q>>::ArgumentBuffer) -> IsNull {
		self.0.as_hyphenated().encode_by_ref(buf)
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for SessionID
where
	DB: Database,
	Hyphenated: sqlx::Decode<'r, DB>,
{
	fn decode(value: <DB as HasValueRef<'r>>::ValueRef) -> Result<Self, BoxDynError> {
		<Hyphenated as sqlx::Decode<'r, DB>>::decode(value)
			.map(Uuid::from)
			.map(Self)
	}
}
