//! The binary entrypoint for the API.

use std::net::SocketAddr;
use std::path::Path;
use std::process::ExitCode;
use std::{fs, io};

use clap::Parser;

/// CS2KZ API.
#[derive(Debug, Parser)]
struct Args
{
	/// Specify a custom config file.
	#[arg(long, default_value = ".config/cs2kz-api.toml")]
	config: Box<Path>,

	/// Override which address the HTTP server is going to listen on.
	#[arg(long)]
	listen_on: Option<SocketAddr>,
}

fn main() -> ExitCode
{
	if dotenvy::dotenv().is_err() {
		eprintln!("WARNING: no `.env` found");
	}

	let args = Args::parse();

	let config = match load_config(&args.config) {
		Ok(config) => config,
		Err(error) => {
			println!("Failed to load config: {error}");
			return ExitCode::FAILURE;
		}
	};

	let runtime = match initialize_runtime(&config.runtime) {
		Ok(runtime) => runtime,
		Err(error) => {
			println!("Failed to build runtime: {error}");
			return ExitCode::FAILURE;
		}
	};

	if let Err(error) = runtime.block_on(cs2kz_api::run(config)) {
		println!("Failed to run API: {error}");
		return ExitCode::FAILURE;
	}

	ExitCode::SUCCESS
}

fn load_config(path: &Path) -> Result<cs2kz_api::Config, Box<dyn std::error::Error>>
{
	let contents = fs::read_to_string(path)?;
	let config = toml::from_str::<cs2kz_api::Config>(&contents)?;

	Ok(config)
}

fn initialize_runtime(
	config: &cs2kz_api::config::runtime::Config,
) -> io::Result<tokio::runtime::Runtime>
{
	let mut runtime = tokio::runtime::Builder::new_multi_thread();

	runtime.enable_all();

	if let Some(n) = config.max_blocking_threads {
		runtime.max_blocking_threads(n.get());
	}

	if let Some(name) = config.worker_thread_name.as_deref() {
		runtime.thread_name(&**name);
	}

	if let Some(size) = config.worker_thread_stack_size {
		runtime.thread_stack_size(size.get());
	}

	if let Some(count) = config.worker_thread_count {
		runtime.worker_threads(count.get());
	}

	runtime.build()
}
