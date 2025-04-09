use {
	crate::config::TracingConfig,
	color_eyre::eyre::{self, WrapErr},
	cs2kz_api::discord,
	std::fs,
	tracing_subscriber::{
		EnvFilter,
		Layer,
		fmt::time::UtcTime,
		layer::SubscriberExt,
		reload,
		util::SubscriberInitExt,
	},
};

pub(crate) struct Guard
{
	_files_guard: Option<tracing_appender::non_blocking::WorkerGuard>,
}

pub(crate) fn init(
	config: &TracingConfig,
) -> eyre::Result<(reload::Handle<Option<discord::TracingLayer>, impl tracing::Subscriber>, Guard)>
{
	let stderr_layer = config.stderr.enable.then(|| {
		tracing_subscriber::fmt::layer()
			.pretty()
			.with_ansi(config.stderr.ansi)
			.with_timer(UtcTime::rfc_3339())
	});

	let (files_layer, files_guard) = config
		.files
		.enable
		.then(|| -> eyre::Result<_> {
			if !config.files.directory.exists() {
				fs::create_dir_all(&*config.files.directory).wrap_err_with(|| {
					format!("failed to create log directory at {:?}", config.files.directory)
				})?;
			}

			let directory =
				config.files.directory.canonicalize().wrap_err_with(|| {
					format!("failed to canonicalize {:?}", config.files.directory)
				})?;

			let (writer, guard) = tracing_appender::rolling::Builder::default()
				.rotation(tracing_appender::rolling::Rotation::DAILY)
				.filename_suffix("log")
				.build(directory)
				.map(tracing_appender::non_blocking)
				.wrap_err("failed to install logfile thread")?;

			let layer = tracing_subscriber::fmt::layer()
				.json()
				.with_file(false)
				.with_line_number(false)
				.with_span_list(true)
				.with_writer(writer)
				.with_timer(UtcTime::rfc_3339());

			Ok((layer, guard))
		})
		.transpose()?
		.unzip();

	let console_layer = config.console.enable.then(|| {
		let builder = console_subscriber::Builder::default();
		let builder = match config.console.server_addr {
			None => builder,
			Some(addr) => builder.server_addr(addr),
		};

		builder.spawn()
	});

	#[cfg(target_os = "linux")]
	let journald_layer = config
		.journald
		.enable
		.then(|| tracing_journald::layer().wrap_err("failed to create tracing-journald layer"))
		.transpose()?;

	let (discord_layer, discord_handle) = reload::Layer::new(None);

	let filtered_layers = Layer::and_then(stderr_layer, files_layer);

	#[cfg(target_os = "linux")]
	let filtered_layers = Layer::and_then(filtered_layers, journald_layer);

	tracing_subscriber::registry()
		.with(filtered_layers.with_filter(EnvFilter::from_default_env()))
		.with(console_layer)
		.with(discord_layer)
		.init();

	Ok((discord_handle, Guard { _files_guard: files_guard }))
}
