//! CLI argument handling.

use std::net::IpAddr;
use std::path::Path;

use clap::Parser;

use crate::config::Config;

/* NOTE:
 * We expose a free function here so callers don't need to have
 * `clap::Parser` in-scope.
 */

/// This is the HTTP server serving the CS2KZ API.
///
/// The API requires a running MariaDB instance it can connect to, as well as a
/// configuration file.
#[derive(Debug, Parser)]
pub struct Args {
	/// Path to the configuration file.
	#[arg(long = "config", default_value = "./cs2kz-api.toml")]
	pub config_path: Box<Path>,

	/// The IP address to listen on.
	///
	/// This option takes precedence over the configuration file.
	#[arg(long)]
	pub ip: Option<IpAddr>,

	/// The port to listen on.
	///
	/// This option takes precedence over the configuration file.
	#[arg(long)]
	pub port: Option<u16>,
}

impl Args {
	/// Applies any relevant config overrides specified as CLI flags in the
	/// given `config` object.
	pub fn apply_to_config(&self, config: &mut Config) {
		if let Some(ip) = self.ip {
			config.http.ip = ip;
		}

		if let Some(port) = self.port {
			config.http.port = port;
		}
	}
}
