use std::io;
use std::net::SocketAddr;

use tokio::net::TcpListener;

use crate::config::ApiConfig;
use crate::database::{self, DatabaseError};
use crate::state::AppState;

#[derive(Debug, Error)]
pub enum ServeError {
	#[error("failed to initialize tokio: {0}")]
	InitializeRuntime(#[source] io::Error),

	#[error("failed to initialize database connection pool: {0}")]
	InitializeDatabaseConnections(#[source] DatabaseError),

	#[error("failed to bind tcp listener: {0}")]
	BindTcpListener(#[source] io::Error),

	#[error("failed to run server: {0}")]
	Axum(#[source] io::Error),
}

/// Serves the API with the given config.
///
/// This function will not return until the API shuts down.
pub fn serve(config: ApiConfig) -> Result<(), ServeError> {
	let mut runtime = tokio::runtime::Builder::new_multi_thread();

	runtime.enable_all();

	if let Some(count) = config.runtime.worker_threads {
		runtime.worker_threads(count.get());
	}

	if let Some(count) = config.runtime.max_blocking_threads {
		runtime.max_blocking_threads(count.get());
	}

	let fut = async {
		if config.tracing.enable {
			use tracing_subscriber::fmt::format::FmtSpan;
			use tracing_subscriber::EnvFilter;

			tracing_subscriber::fmt()
				.pretty()
				.with_span_events(FmtSpan::FULL)
				.with_env_filter(
					EnvFilter::try_from_default_env()
						.unwrap_or_else(|_| EnvFilter::new("cs2kz_api=trace,warn")),
				)
				.init();
		}

		let pool = database::ConnectionPool::new(&config.database)
			.await
			.map_err(ServeError::InitializeDatabaseConnections)?;

		let io = TcpListener::bind(config.http.listen_addr)
			.await
			.map_err(ServeError::BindTcpListener)?;

		let svc = crate::http::router(pool.clone(), config.http.cookies, &config.tracing)
			.with_state(AppState { pool })
			.into_make_service_with_connect_info::<SocketAddr>();

		axum::serve(io, svc).await.map_err(ServeError::Axum)
	};

	runtime
		.build()
		.map_err(ServeError::InitializeRuntime)?
		.block_on(fut)
}
