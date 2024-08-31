//! Tracing layer for logging to journald.

use std::io;

use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to journald.
pub fn layer<S>(
	config: &crate::config::tracing::journald::Config,
) -> io::Result<Option<impl Layer<S>>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	if !config.enable {
		return Ok(None);
	}

	let layer = tracing_journald::layer()
		.map(|layer| layer.with_syslog_identifier(String::from("cs2kz-api")))
		.map(|layer| layer.with_filter(config.env_filter()))?;

	Ok(Some(layer))
}
