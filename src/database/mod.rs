mod connection;
mod error;

use std::num::NonZero;

use futures_util::TryFutureExt;
use sqlx::{
	MySql,
	mysql::{MySqlPool, MySqlPoolOptions},
};
use tracing::{Instrument, trace_span};
use url::Url;

pub use self::{
	connection::DatabaseConnection,
	error::{DatabaseError, DatabaseResult},
};

pub(crate) type QueryBuilder<'args> = sqlx::QueryBuilder<'args, MySql>;

/// A handle to the database
///
/// This can be used to [acquire connections], which are then passed to other
/// functions in this crate.
///
/// [acquire connections]: Database::acquire_connection()
#[must_use]
#[derive(Debug, Clone)]
#[debug("Database(..)")]
pub struct Database
{
	connection_pool: MySqlPool,
}

/// Configuration options for [connecting] to the database
///
/// [connecting]: Database::connect()
#[derive(Debug, Builder)]
pub struct ConnectOptions<'a>
{
	/// The URL of the database we should connect to
	url: &'a Url,

	/// The minimum number of connections to keep in the pool
	#[builder(default = NonZero::<u32>::MIN)]
	min_connections: NonZero<u32>,

	/// The maximum number of connections to keep in the pool
	max_connections: Option<NonZero<u32>>,
}

impl Database
{
	/// Attempts to establish a database connection.
	pub async fn connect(connect_options: ConnectOptions<'_>) -> DatabaseResult<Self>
	{
		let pool_options =
			MySqlPoolOptions::default().min_connections(connect_options.min_connections.get());

		let pool_options = match connect_options.max_connections {
			None => pool_options,
			Some(n) => pool_options.max_connections(n.get()),
		};

		pool_options
			.connect(connect_options.url.as_str())
			.map_ok(|connection_pool| Self { connection_pool })
			.map_err(DatabaseError::from)
			.await
	}

	/// Returns a handle to a connection owned by the pool.
	pub async fn acquire_connection<'c, 'args>(
		&self,
	) -> DatabaseResult<DatabaseConnection<'c, 'args>>
	{
		self.connection_pool
			.acquire()
			.map_ok(DatabaseConnection::new)
			.map_err(DatabaseError::from)
			.instrument(trace_span!("acquire_connection"))
			.await
	}

	/// Executes the given closure `f` inside the context of a transaction.
	pub async fn in_transaction<F, T, E>(&self, f: F) -> Result<T, E>
	where
		F: AsyncFnOnce(&mut DatabaseConnection<'_, '_>) -> Result<T, E>,
		DatabaseError: Into<E>,
	{
		self.acquire_connection()
			.map_err(Into::<E>::into)
			.await?
			.in_transaction(f)
			.await
	}

	/// Closes all open database connections.
	#[tracing::instrument(level = "trace")]
	pub async fn shutdown(self)
	{
		self.connection_pool.close().await;
	}
}
