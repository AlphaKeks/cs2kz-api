use std::{error::Error, num::TryFromIntError};

pub type DatabaseResult<T> = Result<T, DatabaseError>;

/// An error returned by the database
#[derive(Debug, Display, Error, From)]
#[display("database error: {_0}")]
pub struct DatabaseError(sqlx::Error);

impl DatabaseError
{
	/// Helper function construct a [`DatabaseError`] from a failed attempt to
	/// parse the result of a `SELECT COUNT(…) AS count FROM …` query.
	pub(crate) fn convert_count(error: TryFromIntError) -> Self
	{
		Self::decode_column("count", error)
	}

	pub(crate) fn decode_column(
		column: impl Into<String>,
		error: impl Into<Box<dyn Error + Send + Sync>>,
	) -> Self
	{
		Self(sqlx::Error::ColumnDecode { index: column.into(), source: error.into() })
	}

	/// Returns whether this error is a unique key violation of the given `key`.
	pub fn is_unique_violation(&self, key: &str) -> bool
	{
		self.0
			.as_database_error()
			.is_some_and(|error| error.is_unique_violation() && error.message().contains(key))
	}
}
