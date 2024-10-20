use problem_details::AsProblemDetails;

use crate::database::ErrorKind;
use crate::http::problem_details::Problem;

/// A database error.
#[derive(Debug, Error)]
#[debug("{_0:?}")]
#[error("database error: {_0}")]
pub struct DatabaseError(#[from] sqlx::Error);

impl DatabaseError {
	/// Returns the message associated with this error, as returned by the database.
	pub fn message(&self) -> Option<&str> {
		self.0.as_database_error().map(|error| error.message())
	}

	/// Classifies this error.
	pub fn kind(&self) -> ErrorKind {
		self.0
			.as_database_error()
			.map_or(ErrorKind::Other, |error| error.kind())
	}

	/// Checks if this error was caused by a `PK` / `UNIQUE` constraint violation.
	pub fn is_unique_violation(&self) -> bool {
		matches!(self.kind(), ErrorKind::UniqueViolation)
	}

	/// Checks if this error was caused by a foreign key constraint violation.
	pub fn is_fk_violation(&self, fk: &str) -> bool {
		self.0.as_database_error().is_some_and(|error| {
			error.kind() == ErrorKind::ForeignKeyViolation && error.message().contains(fk)
		})
	}
}

impl AsProblemDetails for DatabaseError {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::Internal
	}
}
