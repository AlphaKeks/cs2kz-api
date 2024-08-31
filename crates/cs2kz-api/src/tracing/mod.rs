use std::io;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::Layer;

mod stderr;
mod files;
mod console;

#[cfg(target_os = "linux")]
mod journald;

pub fn init(config: &crate::config::tracing::Config) -> io::Result<Option<WorkerGuard>>
{
	if !config.enable {
		return Ok(None);
	}

	let stderr = config.stderr.as_ref().and_then(stderr::layer);
	let (files, guard) = config
		.files
		.as_ref()
		.map(files::layer)
		.transpose()?
		.flatten()
		.unzip();

	let layer = Layer::and_then(stderr, files);

	#[cfg(target_os = "linux")]
	let layer = {
		let journald = config
			.journald
			.as_ref()
			.map(journald::layer)
			.transpose()?
			.flatten();

		layer.and_then(journald)
	};

	tracing_subscriber::registry()
		.with(layer.with_filter(config.env_filter()))
		.with(config.console.as_ref().and_then(console::layer))
		.init();

	info!("initialized tracing");

	Ok(guard)
}
