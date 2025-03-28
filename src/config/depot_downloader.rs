use std::path::{Path, PathBuf};

#[derive(Debug, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub(crate) struct DepotDownloaderConfig
{
	#[serde(default = "default_exe_path")]
	pub exe_path: Box<Path>,

	#[serde(default = "default_out_dir")]
	pub out_dir: Box<Path>,
}

impl Default for DepotDownloaderConfig
{
	fn default() -> Self
	{
		Self { exe_path: default_exe_path(), out_dir: default_out_dir() }
	}
}

fn default_exe_path() -> Box<Path>
{
	PathBuf::from("DepotDownloader").into_boxed_path()
}

fn default_out_dir() -> Box<Path>
{
	PathBuf::from("/var/lib/cs2kz-api/workshop").into_boxed_path()
}
