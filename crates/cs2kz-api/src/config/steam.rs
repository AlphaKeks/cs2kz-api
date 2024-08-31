use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Steam WebAPI key.
	pub api_key: Box<str>,

	/// Directory to use for storing downloaded workshop assets (e.g. map files).
	pub workshop_asset_dir: Box<Path>,

	/// Path to a `DepotDownloader` executable.
	pub depot_downloader: Box<Path>,
}
