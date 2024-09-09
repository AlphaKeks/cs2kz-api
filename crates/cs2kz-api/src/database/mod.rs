#![allow(unused)]

use std::num::NonZero;
use std::thread;

use cs2kz::SteamID;
use url::Url;

mod error_ext;
pub use error_ext::ErrorExt;

pub type DB = sqlx::MySql;

pub type Row = <DB as sqlx::Database>::Row;
pub type Connection = <DB as sqlx::Database>::Connection;
pub type ConnectOptions = <<DB as sqlx::Database>::Connection as sqlx::Connection>::Options;

pub type Pool = sqlx::Pool<DB>;
pub type QueryBuilder<'args> = sqlx::QueryBuilder<'args, DB>;
pub type Transaction<'c> = sqlx::Transaction<'c, DB>;

pub type Error = sqlx::Error;
pub type DatabaseError = sqlx::mysql::MySqlDatabaseError;
pub type Result<T, E = Error> = std::result::Result<T, E>;

pub use sqlx::mysql::MySqlExecutor as Executor;
pub use sqlx::types::Json;

#[instrument(fields(database_url = %database_url))]
pub async fn connect(
	database_url: &Url,
	min_connections: u32,
	max_connections: Option<NonZero<u32>>,
) -> Result<Pool>
{
	if let Some(max_connections) = max_connections.map(NonZero::get) {
		assert!(
			max_connections > min_connections,
			"`database.max-connections` must be greater than `database.min-connections`"
		);
	}

	let max_connections = max_connections.map_or_else(get_core_count, NonZero::get);

	sqlx::pool::PoolOptions::<DB>::new()
		.min_connections(min_connections)
		.max_connections(max_connections)
		.connect(database_url.as_str())
		.await
}

fn get_core_count() -> u32
{
	thread::available_parallelism()
		.expect("failed to query cpu core count")
		.get()
		.try_into()
		.expect("too many cpu cores")
}
