//! Wrappers and re-exports for [`sqlx`].

use std::ops;

use axum::extract::{FromRef, FromRequestParts};
use sqlx::pool::{PoolConnection, PoolOptions};
use sqlx::{Database, MySql};

use crate::config::DatabaseConfig;
use crate::http::response::ErrorResponse;

mod error;
pub use error::DatabaseError;

mod row_stream;
pub use row_stream::RowStream;

mod uuid_as_bytes;
pub(crate) use uuid_as_bytes::uuid_as_bytes;

/// The database driver we're using.
pub type Driver = MySql;

/// Common error classifications.
pub type ErrorKind = sqlx::error::ErrorKind;

/// A result type specialized for database errors.
pub type Result<T, E = DatabaseError> = std::result::Result<T, E>;

/// A live database transaction.
///
/// A transaction that has not been explicitly committed or aborted will be
/// aborted on drop.
pub type Transaction<'conn, DB = Driver> = sqlx::Transaction<'conn, DB>;

/// The API's database migrations.
pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!();

/// Initializes a new connection pool.
#[instrument(err)]
pub async fn connect(config: &DatabaseConfig) -> Result<ConnectionPool> {
	PoolOptions::new()
		.min_connections(config.min_connections)
		.max_connections(config.max_connections.get())
		.connect(config.url.as_str())
		.await
		.map(ConnectionPool)
		.map_err(Into::into)
}

/// A pool of database [`Connection`]s.
#[derive(Debug)]
pub struct ConnectionPool<DB = Driver>(sqlx::Pool<DB>)
where
	DB: Database;

impl<DB> ConnectionPool<DB>
where
	DB: Database,
{
	/// Gets a [`Connection`] from the pool.
	pub async fn get_connection(&self) -> Result<Connection<DB>> {
		self.0.acquire().await.map(Connection).map_err(Into::into)
	}
}

impl<DB> Clone for ConnectionPool<DB>
where
	DB: Database,
{
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}

	fn clone_from(&mut self, other: &Self) {
		self.0.clone_from(&other.0)
	}
}

/// A live database connection.
#[derive(Debug)]
pub struct Connection<DB = Driver>(PoolConnection<DB>)
where
	DB: Database;

impl<DB> ops::Deref for Connection<DB>
where
	DB: Database,
{
	type Target = DB::Connection;

	fn deref(&self) -> &Self::Target {
		&*self.0
	}
}

impl<DB> ops::DerefMut for Connection<DB>
where
	DB: Database,
{
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut *self.0
	}
}

impl<DB> AsRef<DB::Connection> for Connection<DB>
where
	DB: Database,
{
	fn as_ref(&self) -> &DB::Connection {
		self
	}
}

impl<DB> AsMut<DB::Connection> for Connection<DB>
where
	DB: Database,
{
	fn as_mut(&mut self) -> &mut DB::Connection {
		self
	}
}

impl<S, DB> FromRequestParts<S> for Connection<DB>
where
	S: Send + Sync,
	DB: Database,
	ConnectionPool<DB>: FromRef<S>,
{
	type Rejection = ErrorResponse;

	async fn from_request_parts(
		_parts: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		ConnectionPool::<DB>::from_ref(state)
			.get_connection()
			.await
			.map_err(Into::into)
	}
}
