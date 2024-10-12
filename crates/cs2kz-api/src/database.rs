//! Re-exports from [`sqlx`] relevant to our specific database ([`DB`]), as
//! well as [an entrypoint into the database](connect).
//!
//! [`MIGRATIONS`] can be used to apply the database migrations.

use std::future::Future;
use std::num::NonZero;
use std::thread;

use sqlx::migrate::Migrator;
use sqlx::pool::PoolOptions;
use sqlx::Database;
use url::Url;

pub mod macros;

mod error_ext;
pub use error_ext::ErrorExt;

mod row_stream;
pub use row_stream::RowStream;

/// The database driver we're using.
pub type DB = sqlx::MySql;

pub type Error = sqlx::Error;
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// A live connection to the database.
pub type Connection = <DB as Database>::Connection;

/// A pool of database connections.
pub type ConnectionPool = sqlx::Pool<DB>;

/// An in-progress transaction.
pub type Transaction<'conn> = sqlx::Transaction<'conn, DB>;

/// The API's database migrations.
///
/// These are compiled in and can be executed using a [`Connection`] via [`Migrator::run()`].
pub static MIGRATIONS: Migrator = sqlx::migrate!();

/// Creates a [`ConnectionPool`] with the provided bounds on connection limits.
pub fn connect(
	url: &Url,
	min_connections: u32,
	max_connections: Option<NonZero<u32>>,
) -> impl Future<Output = Result<ConnectionPool>> + '_ {
	fn n_cpus() -> u32 {
		thread::available_parallelism().map_or(1, |count| count.get() as u32)
	}

	PoolOptions::new()
		.min_connections(min_connections)
		.max_connections(max_connections.map_or_else(n_cpus, NonZero::get))
		.connect(url.as_str())
}
