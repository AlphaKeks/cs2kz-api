use std::fs;
use std::path::Path;

use anyhow::Context;

mod cli;

fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.pretty()
		.with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
		.init();

	let cli_args = cli::args();
	let mut config = if let Some(config_path) = cli_args.config_path.as_deref() {
		read_and_parse_config_file(config_path)?
	} else if fs::exists("./cs2kz-api.toml")? {
		read_and_parse_config_file(Path::new("./cs2kz-api.toml"))?
	} else {
		cs2kz_api::Config::default()
	};

	cli_args.apply_to_config(&mut config);

	cs2kz_api::run(config).context("failed to run API")
}

fn read_and_parse_config_file(path: &Path) -> anyhow::Result<cs2kz_api::Config> {
	fs::read_to_string(path)
		.context("failed to read configuration file")
		.and_then(|text| toml::from_str(&text).context("failed to parse configuration file"))
}
