//! Facade to the database driver
//!
//! The API uses [MariaDB] as its backing database. The types in this module
//! serve as handles to connections, connection pools, transactions, etc. and
//! are passed into various functions in this crate.

pub use self::{
	connection::Connection,
	error::{DatabaseError, DatabaseResult, MigrationError},
};
use {
	futures_util::TryFutureExt,
	sqlx::{
		MySql,
		migrate::Migrator,
		mysql::{MySqlPool, MySqlPoolOptions},
	},
	std::{fmt, num::NonZero},
	url::Url,
};

mod connection;
mod error;

pub(crate) type QueryBuilder<'args> = sqlx::QueryBuilder<'args, MySql>;

static MIGRATIONS: Migrator = sqlx::migrate!();

/// A pool of [`Connection`]s
///
/// This can be used to [acquire individual connections], which are then passed
/// to other functions in this crate.
///
/// [acquire connections]: ConnectionPool::acquire()
#[must_use]
#[derive(Clone)]
pub struct ConnectionPool
{
	inner: MySqlPool,
}

#[bon::bon]
impl ConnectionPool
{
	/// Attempts to establish a database connection.
	#[builder]
	pub fn new(
		/// The URL of the database we should connect to
		url: &Url,

		/// The minimum number of connections to keep in the pool
		#[builder(default = NonZero::<u32>::MIN)]
		min_connections: NonZero<u32>,

		/// The maximum number of connections to keep in the pool
		max_connections: Option<NonZero<u32>>,
	) -> impl Future<Output = DatabaseResult<Self>>
	{
		let pool_options = MySqlPoolOptions::default().min_connections(min_connections.get());
		let pool_options = match max_connections {
			None => pool_options,
			Some(n) => pool_options.max_connections(n.get()),
		};

		pool_options
			.connect(url.as_str())
			.map_ok(|connection_pool| Self { inner: connection_pool })
			.map_err(DatabaseError::from)
	}

	/// Runs outstanding database migrations.
	#[instrument(level = "trace", skip(self), err(level = "warn"))]
	pub async fn run_migrations(&self) -> Result<(), MigrationError>
	{
		MIGRATIONS.run(&self.inner).map_err(MigrationError::from).await
	}

	/// Acquires a handle to a connection in the pool.
	///
	/// Because the handle is owned, the `'c` lifetime may be chosen freely[^c].
	///
	///
	/// [^c]: usually `'c` will be `'static` but making it generic helps
	///       with type inference
	#[instrument(level = "trace", skip(self), err(level = "warn"))]
	pub async fn acquire<'c, 'env>(&self) -> DatabaseResult<Connection<'c, 'env>>
	{
		self.inner
			.acquire()
			.map_ok(Connection::from_raw)
			.map_err(DatabaseError::from)
			.await
	}

	/// Executes an `async` closure `f` inside the context of a transaction.
	///
	/// If the closure returns `Ok` the transaction is committed.
	/// If the closure returns `Err` the transaction is rolled back.
	#[instrument(level = "trace", skip_all, err(level = "debug"))]
	pub async fn in_transaction<F, T, E>(&self, f: F) -> Result<T, E>
	where
		F: AsyncFnOnce(&mut Connection<'_, '_>) -> Result<T, E>,
		E: fmt::Display,
		DatabaseError: Into<E>,
	{
		self.acquire().map_err(Into::<E>::into).await?.in_transaction(f).await
	}

	/// Closes all open database connections.
	#[instrument(level = "trace")]
	pub async fn shutdown(self)
	{
		self.inner.close().await;
	}
}

impl fmt::Debug for ConnectionPool
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_tuple("ConnectionPool").finish_non_exhaustive()
	}
}
