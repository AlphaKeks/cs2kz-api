use std::path::Path;
use std::{fs, io, process};

use cs2kz_api::config::ApiConfig;
use thiserror::Error;

mod cli;

fn main() -> process::ExitCode {
	if let Err(error) = main_inner() {
		eprintln!("{error}");
		return process::ExitCode::FAILURE;
	}

	process::ExitCode::SUCCESS
}

#[derive(Debug, Error)]
enum Error {
	#[error("failed to read configuration file: {0}")]
	ReadConfigFile(#[source] io::Error),

	#[error("failed to parse configuration file: {0}")]
	ParseConfigFile(#[source] toml::de::Error),

	#[error("failed to serve API: {0}")]
	ServeApi(#[source] cs2kz_api::ServeError),
}

fn main_inner() -> Result<(), Error> {
	match cli::args().action {
		cli::Action::Serve {
			config,
			ip_addr,
			port,
		} => {
			let mut config = get_config(config.as_deref())?;

			if let Some(ip_addr) = ip_addr {
				config.http.listen_addr.set_ip(ip_addr);
			}

			if let Some(port) = port {
				config.http.listen_addr.set_port(port);
			}

			cs2kz_api::serve(config).map_err(Error::ServeApi)
		}
	}
}

fn get_config(path: Option<&Path>) -> Result<ApiConfig, Error> {
	match fs::read_to_string(path.unwrap_or(Path::new("cs2kz-api.toml"))) {
		Ok(text) => toml::from_str(&text).map_err(Error::ParseConfigFile),
		Err(error) if error.kind() == io::ErrorKind::NotFound && path.is_some() => {
			Err(Error::ReadConfigFile(error))
		}
		Err(_) => Ok(ApiConfig::default()),
	}
}
