//! Tracing layer for logging to tokio-console.

use console_subscriber::ConsoleLayer;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Creates a tracing layer that will emit logs to tokio-console.
pub fn layer<S>(config: &crate::config::tracing::console::Config) -> Option<impl Layer<S>>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	if !config.enable {
		return None;
	}

	let layer = ConsoleLayer::builder()
		.server_addr(config.server_addr.clone())
		.spawn();

	Some(layer)
}
