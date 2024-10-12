use sqlx::error::ErrorKind;

/// An extension trait for [`sqlx::Error`] that makes checking common error conditions easier.
pub trait ErrorExt {
	/// Checks if the error message contains some string `s`.
	fn message_contains(&self, s: &str) -> bool;

	/// Checks if this error was caused by violating a `UNIQUE` constraint, or inserting
	/// a duplicate primary key.
	fn is_unique_violation(&self) -> bool;

	/// Checks if this error was caused by a foreign key violation of the specified key.
	fn is_fk_violation(&self, fk: &str) -> bool;
}

impl ErrorExt for sqlx::Error {
	fn message_contains(&self, s: &str) -> bool {
		self.as_database_error()
			.is_some_and(|error| error.message().contains(s))
	}

	fn is_unique_violation(&self) -> bool {
		self.as_database_error()
			.is_some_and(|error| error.kind() == ErrorKind::UniqueViolation)
	}

	fn is_fk_violation(&self, fk: &str) -> bool {
		self.as_database_error().is_some_and(|error| {
			error.kind() == ErrorKind::ForeignKeyViolation && error.message().contains(fk)
		})
	}
}
