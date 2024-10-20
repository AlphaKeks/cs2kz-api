use sqlx::pool::{PoolConnection, PoolOptions};

use crate::config::DatabaseConfig;
use crate::database::{self, Transaction};

/// A pool of database connections.
#[derive(Debug)]
#[debug("ConnectionPool")]
pub struct ConnectionPool<DB = database::Driver>(sqlx::Pool<DB>)
where
	DB: sqlx::Database;

impl<DB> ConnectionPool<DB>
where
	DB: sqlx::Database,
{
	/// Initializes a new pool by connecting to the database at the given `url`.
	///
	/// `min_connections` connections will be opened (and kept around) immediately.
	/// The pool will never open more than `max_connections`. If `max_connections` is [`None`],
	/// the pool will use the amount of available CPUs instead.
	#[instrument(err)]
	pub async fn new(config: &DatabaseConfig) -> database::Result<Self> {
		let pool = PoolOptions::new()
			.min_connections(config.min_connections)
			.max_connections(config.max_connections.get())
			.connect(config.url.as_str())
			.await?;

		Ok(Self(pool))
	}

	/// Gets a connection from the pool.
	///
	/// The returned [`PoolConnection`] will be returned to the pool automatically on drop.
	#[instrument(level = "trace", skip_all, err)]
	pub async fn get_connection(&self) -> database::Result<PoolConnection<DB>> {
		self.0.acquire().await.map_err(Into::into)
	}

	/// Begins a transaction.
	///
	/// The returned [`Transaction`] will be rolled back automatically on drop unless
	/// [`Transaction::commit()`] / [`Transaction::rollback()`] is called explicitly
	/// beforehand.
	#[instrument(level = "trace", skip_all, err)]
	pub async fn begin_transaction(&self) -> database::Result<Transaction<'static, DB>> {
		self.0.begin().await.map_err(Into::into)
	}
}

impl<DB> Clone for ConnectionPool<DB>
where
	DB: sqlx::Database,
	sqlx::Pool<DB>: Clone,
{
	fn clone(&self) -> Self {
		Self(self.0.clone())
	}

	fn clone_from(&mut self, source: &Self) {
		self.0.clone_from(&source.0);
	}
}
