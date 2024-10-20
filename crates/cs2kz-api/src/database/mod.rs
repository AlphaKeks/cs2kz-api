//! Wrappers and re-exports for [`sqlx`].

mod error;
pub use error::DatabaseError;

mod pool;
pub use pool::ConnectionPool;

mod row_stream;
pub use row_stream::RowStream;

/// The database driver we're using.
pub type Driver = sqlx::MySql;

/// Common error classifications.
pub type ErrorKind = sqlx::error::ErrorKind;

/// A result type specialized for database errors.
pub type Result<T, E = DatabaseError> = std::result::Result<T, E>;

/// A live database connection.
pub type Connection<DB = Driver> = <DB as sqlx::Database>::Connection;

/// A live database transaction.
///
/// A transaction that has not been explicitly committed or aborted will be aborted on drop.
pub type Transaction<'conn, DB = Driver> = sqlx::Transaction<'conn, DB>;

/// The API's database migrations.
#[expect(dead_code)]
pub static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!();
