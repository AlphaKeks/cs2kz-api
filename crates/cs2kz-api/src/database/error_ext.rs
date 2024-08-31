/// Extension trait for SQL errors.
pub trait ErrorExt
{
	/// Checks if the error was caused by a `UNIQUE` / PK constraint failure.
	fn is_duplicate(&self) -> bool;

	/// Checks if the error was caused by a failed `CHECK()`.
	fn is_check_violation(&self, check_name: &str) -> bool;
}

impl ErrorExt for super::Error
{
	fn is_duplicate(&self) -> bool
	{
		self.as_database_error()
			.is_some_and(|error| error.is_unique_violation())
	}

	fn is_check_violation(&self, check_name: &str) -> bool
	{
		self.as_database_error().map_or(false, |error| {
			error.is_check_violation() && error.message().contains(check_name)
		})
	}
}
