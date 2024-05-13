//! The CS2KZ API.

#![allow(clippy::missing_panics_doc, clippy::missing_docs_in_private_items)]

use anyhow::Context;
use cs2kz_api::API;
use dotenvy::dotenv;
use sqlx::pool::PoolOptions;
use sqlx::MySql;
use tokio::net::TcpListener;

mod logging;

/// The minimum amount of open database connections to keep in the connection pool.
const MIN_DB_CONNECTIONS: u32 = if cfg!(feature = "production") {
	200
} else {
	20
};

/// The maximum amount of open database connections to keep in the connection pool.
const MAX_DB_CONNECTIONS: u32 = if cfg!(feature = "production") {
	256
} else {
	50
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	if dotenv().is_err() {
		eprintln!("WARNING: no `.env` file found");
	}

	let _guard = logging::init().context("initialize logging")?;

	let config = cs2kz_api::Config::new().context("initialize API config")?;

	let database = PoolOptions::<MySql>::new()
		.min_connections(MIN_DB_CONNECTIONS)
		.max_connections(MAX_DB_CONNECTIONS)
		.connect(config.database_url.as_str())
		.await
		.context("connect to database")?;

	let tcp_listener = TcpListener::bind(config.addr)
		.await
		.context("bind to tcp socket")?;

	API::new(config, database, tcp_listener)
		.await
		.context("initialize API")?
		.run()
		.await
		.context("run API")?;

	Ok(())
}
