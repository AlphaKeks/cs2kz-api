#![allow(missing_copy_implementations, reason = "configs won't be copied around")]

pub(crate) use self::{
	access_keys::AccessKeys,
	database::DatabaseConfig,
	depot_downloader::DepotDownloaderConfig,
	http::HttpConfig,
	runtime::RuntimeConfig,
	steam::SteamConfig,
	tracing::TracingConfig,
};
use {
	color_eyre::{
		Section,
		eyre::{self, WrapErr},
	},
	cs2kz_api::{discord, email, server_monitor},
	serde::Deserialize,
	std::{fs, path::Path},
};

mod access_keys;
mod database;
mod depot_downloader;
mod http;
mod runtime;
mod steam;
pub(crate) mod tracing;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct Config
{
	pub runtime: RuntimeConfig,
	pub tracing: TracingConfig,
	pub database: DatabaseConfig,
	pub http: HttpConfig,
	pub steam: SteamConfig,
	pub access_keys: AccessKeys,
	pub depot_downloader: DepotDownloaderConfig,
	pub server_monitor: Option<server_monitor::Config>,
	pub discord: Option<discord::Config>,
	pub email: Option<email::client::Config>,
}

impl Config
{
	pub(crate) fn load_from_file(path: impl AsRef<Path>) -> eyre::Result<Self>
	{
		let file = fs::read_to_string(path.as_ref())
			.wrap_err_with(|| format!("failed to read configuration file at {:?}", path.as_ref()))
			.suggestion("create the file or run with `--config` to specify an alternative path")?;

		toml::from_str(&file).wrap_err("failed to parse configuration file")
	}
}
