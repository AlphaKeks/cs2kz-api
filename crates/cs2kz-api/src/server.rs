//! The HTTP server.

use std::io;
use std::net::SocketAddr;

use clap::Parser;
use tokio::net::TcpListener;
use tokio::runtime::{self, Runtime};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::FmtSpan;

use crate::config::{Config, RuntimeConfig, TracingConfig};
use crate::database::DatabaseError;
use crate::state::AppState;
use crate::{cli, config, database};

/// Errors that can occur when running the server.
#[derive(Debug, Error)]
#[expect(missing_docs)]
pub enum Error {
	#[error(transparent)]
	ParseCliArguments(#[from] clap::error::Error),

	#[error(transparent)]
	LoadConfiguration(#[from] config::LoadFromFileError),

	#[error("failed to initialize tokio: {0}")]
	InitializeTokio(#[source] io::Error),

	#[error("failed to establish database connection: {0}")]
	Database(#[from] DatabaseError),

	#[error("failed to bind tcp socket: {0}")]
	BindTcpSocket(#[source] io::Error),

	#[error("failed to get tcp local addr: {0}")]
	GetTcpLocalAddr(#[source] io::Error),

	#[error("failed to run http server: {0}")]
	Serve(#[source] io::Error),
}

/// Runs the server.
pub fn run() -> Result<(), Error> {
	let args = cli::Args::try_parse()?;
	let mut config = Config::load_from_file(&*args.config_path)?;
	args.apply_to_config(&mut config);

	if config.tracing.enable {
		initialize_tracing(&config.tracing);
	}

	initialize_runtime(&config.runtime)?.block_on(async {
		let database = database::connect(&config.database).await?;
		let tcp_listener = TcpListener::bind(config.http.socket_addr())
			.await
			.map_err(Error::BindTcpSocket)?;

		let local_addr = tcp_listener.local_addr().map_err(Error::GetTcpLocalAddr)?;

		info!("listening on '{local_addr}'");

		let service = crate::http::router(database.clone(), config)
			.with_state(AppState { database })
			.into_make_service_with_connect_info::<SocketAddr>();

		axum::serve(tcp_listener, service)
			.await
			.map_err(Error::Serve)
	})
}

fn initialize_tracing(config: &TracingConfig) {
	assert!(config.enable, "called `initialize_tracing` even though tracing was disabled");

	tracing_subscriber::fmt()
		.pretty()
		.with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
		.with_env_filter(EnvFilter::from_default_env())
		.init();
}

fn initialize_runtime(config: &RuntimeConfig) -> Result<Runtime, Error> {
	let mut runtime = runtime::Builder::new_multi_thread();

	if let Some(count) = config.worker_threads {
		runtime.worker_threads(count.get());
	}

	runtime.enable_all().build().map_err(Error::InitializeTokio)
}
