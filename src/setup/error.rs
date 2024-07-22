//! Errors that can occur while initializing everything.

use thiserror::Error;

/// The different errors that can happen in [`server()`].
///
/// [`server()`]: crate::server
#[derive(Debug, Error)]
pub enum Error
{
	/// Something went wrong connecting to the database.
	#[error("failed to setup database: {0}")]
	Database(#[from] sqlx::Error),

	/// Something went wrong initializing the JWT part of the auth service.
	#[error("failed to setup jwt state: {0}")]
	Jwt(#[from] jsonwebtoken::errors::Error),

	/// Something went wrong applying database migrations.
	#[error("failed to run migrations: {0}")]
	Migrations(#[from] sqlx::migrate::MigrateError),
}
