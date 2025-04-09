pub(crate) use self::environment::Environment;
use {
	crate::config::RuntimeConfig,
	std::io,
	tokio::runtime::{Builder, Runtime},
};

pub(crate) mod environment;

pub(crate) fn build(config: &RuntimeConfig) -> io::Result<Runtime>
{
	let mut builder = Builder::new_multi_thread();

	builder.enable_time();
	builder.enable_io();

	if let Some(n) = config.worker_threads {
		builder.worker_threads(n.get());
	}

	if let Some(n) = config.max_blocking_threads {
		builder.max_blocking_threads(n.get());
	}

	builder.build()
}
