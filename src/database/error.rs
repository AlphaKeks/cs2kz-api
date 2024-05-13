//! Extension traits for dealing with SQL errors.

/// Extension trait for dealing with SQL errors.
pub trait SqlxErrorExt {
	/// Checks if the error is a "duplicate entry" error.
	fn is_duplicate_entry(&self) -> bool;

	/// Checks if the error is a foreign key violation (for a specific key).
	fn is_fk_violation(&self, fk: &str) -> bool;
}

impl SqlxErrorExt for sqlx::Error {
	fn is_duplicate_entry(&self) -> bool {
		self.as_database_error()
			.is_some_and(|err| err.code().as_deref() == Some("23000"))
	}

	fn is_fk_violation(&self, fk: &str) -> bool {
		self.as_database_error().is_some_and(|err| {
			err.is_foreign_key_violation()
				&& err.message().contains(&format!("FOREIGN KEY (`{fk}`)"))
		})
	}
}
