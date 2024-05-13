use anyhow::Context;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod stderr;
mod files;

/// Initializes the global tracing subscriber.
pub fn init() -> anyhow::Result<WorkerGuard> {
	let stderr = stderr::layer();
	let (files, guard, log_dir) = files::layer().context("log files layer")?;
	let subscriber = tracing_subscriber::registry().with(stderr).with(files);

	subscriber.init();

	info!(target: "audit_log", ?log_dir, "initialized logging");

	Ok(guard)
}
