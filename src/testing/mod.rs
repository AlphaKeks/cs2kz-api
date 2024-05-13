#![allow(clippy::needless_pub_self)]
#![allow(dead_code, unused_macros, unused_imports)] // FIXME

use std::fmt::Display;

use anyhow::Context as _;
use derive_more::Debug;
use rand::{thread_rng, Rng};
use sqlx::migrate::MigrateDatabase;
use sqlx::pool::PoolOptions;
use sqlx::MySql;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task;
use tracing::debug;
use url::Url;
use uuid::Uuid;

use crate::{Config, API};

mod hello_world;

pub type TestResult = anyhow::Result<()>;

/// Testing Context.
///
/// A reference to an instance of this struct is passed to every test, and can be used to access
/// the API's database, and HTTP client, and has other useful helper methods.
#[derive(Debug)]
pub struct Context {
	/// The unique ID for this test.
	#[debug("{test_id}")]
	test_id: Uuid,

	/// The API's configuration.
	#[debug(skip)]
	config: Config,

	/// The API's database connection pool.
	#[debug(skip)]
	database: sqlx::Pool<MySql>,

	/// HTTP client for making API requests.
	#[debug(skip)]
	http_client: reqwest::Client,

	/// Shutdown signal for the API.
	shutdown: oneshot::Sender<()>,
}

impl Context {
	/// The ID of the current test.
	pub const fn id(&self) -> Uuid {
		self.test_id
	}

	/// The API's configuration.
	pub const fn config(&self) -> &Config {
		&self.config
	}

	/// The API's database connection pool.
	pub const fn database(&self) -> &sqlx::Pool<MySql> {
		&self.database
	}

	/// An HTTP client.
	pub const fn http_client(&self) -> &reqwest::Client {
		&self.http_client
	}

	/// Create a URL for making an API request.
	///
	/// # Panics
	///
	/// This function will panic if joining the base URL with the given `path`
	/// results in an invalid URL.
	pub fn url<P>(&self, path: P) -> Url
	where
		P: Display,
	{
		Url::parse(&format!("http://{}", self.config.addr))
			.expect("valid url")
			.join(&path.to_string())
			.expect("valid path")
	}

	/// Creates a new testing context.
	///
	/// This is used by macro code and should not be invoked manually.
	#[doc(hidden)]
	pub async fn new() -> anyhow::Result<Self> {
		let test_id = Uuid::now_v7();
		let mut config = Config::new().context("initialize API config")?;

		config.addr.set_port(thread_rng().gen_range(5000..=50000));
		config
			.public_url
			.set_port(Some(config.addr.port()))
			.ok()
			.context("tests must use a custom port")?;

		let old_user = config.database_url.username().to_owned();

		config
			.database_url
			.set_username("root")
			.ok()
			.context("tests must be able to access the database as root")?;

		config
			.database_url
			.path_segments_mut()
			.ok()
			.context("database url must have a base")?
			.pop()
			.push(&format!("cs2kz-{test_id}"));

		MySql::drop_database(config.database_url.as_str())
			.await
			.context("drop old database")?;

		debug!(%test_id, "creating test database");

		MySql::create_database(config.database_url.as_str())
			.await
			.with_context(|| format!("create test database `{test_id}`"))?;

		config
			.database_url
			.set_username(&old_user)
			.ok()
			.context("reset db username")?;

		debug!(%test_id, "connecting to database");
		let database = PoolOptions::<MySql>::new()
			.connect(config.database_url.as_str())
			.await
			.context("connect to database")?;

		debug!(%test_id, "running migrations");
		sqlx::migrate!("./database/migrations")
			.run(&database)
			.await
			.context("run migrations")?;

		let http_client = reqwest::Client::new();
		let (shutdown, rx) = oneshot::channel();

		let ctx = Self {
			test_id,
			config: config.clone(),
			database: database.clone(),
			http_client,
			shutdown,
		};

		let tcp_listener = TcpListener::bind(config.addr)
			.await
			.context("bind to tcp socket")?;

		task::spawn(async move {
			API::new(config, database, tcp_listener)
				.await
				.context("initialize API")?
				.run_until(async move {
					_ = rx.await;
				})
				.await?;

			anyhow::Ok(())
		});

		Ok(ctx)
	}

	/// Performs cleanup after a test run.
	///
	/// This is used by macro code and should not be invoked manually.
	#[doc(hidden)]
	pub async fn cleanup(mut self) -> anyhow::Result<()> {
		let Context { test_id, .. } = self;

		debug!(%test_id, "cleaning up");

		if self.shutdown.send(()).is_err() {
			anyhow::bail!("API shut down already?");
		}

		self.config
			.database_url
			.set_username("root")
			.ok()
			.context("tests must be able to access the database as root")?;

		MySql::drop_database(self.config.database_url.as_str())
			.await
			.with_context(|| format!("drop test database `{test_id}`"))?;

		Ok(())
	}
}

macro_rules! assert {
	($cond:expr) => {
		::anyhow::ensure!($cond, "assertion failed: `{}`", stringify!($cond));
	};

	($cond:expr, $($t:tt)*) => {
		::anyhow::ensure!($cond, "assertion failed: {}", $($t)*);
	};
}

pub(self) use assert;

macro_rules! assert_eq {
	($left:expr, $right:expr) => {
		::anyhow::ensure!(
			&$left == &$right,
			"assertion failed: `left == right`\n  left: {}\n  right: {}",
			&$left,
			&$right
		);
	};
}

pub(self) use assert_eq;

macro_rules! assert_ne {
	($left:expr, $right:expr) => {
		::anyhow::ensure!(
			&$left != &$right,
			"assertion failed: `left != right`\n  left: {}\n  right: {}",
			&$left,
			&$right
		);
	};
}

pub(self) use assert_ne;
