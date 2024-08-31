//! Tracing layer for logging to files.

use std::{fs, io};

use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to files.
pub fn layer<S>(
	config: &crate::config::tracing::files::Config,
) -> io::Result<Option<(impl Layer<S>, WorkerGuard)>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	if !config.enable {
		return Ok(None);
	}

	if !config.directory.exists() {
		fs::create_dir_all(&config.directory)?;
	}

	let log_dir = config.directory.canonicalize()?;
	let (writer, guard) = tracing_appender::rolling::Builder::new()
		.rotation(Rotation::DAILY)
		.filename_suffix("log")
		.build(&log_dir)
		.map(tracing_appender::non_blocking)
		.map_err(io::Error::other)?;

	let layer = tracing_subscriber::fmt::layer()
		.json()
		.with_file(true)
		.with_level(true)
		.with_line_number(true)
		.with_span_events(FmtSpan::FULL)
		.with_target(true)
		.with_thread_ids(true)
		.with_thread_names(true)
		.with_writer(writer)
		.with_filter(config.env_filter());

	Ok(Some((layer, guard)))
}
