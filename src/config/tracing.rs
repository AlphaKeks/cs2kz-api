use {
	serde::Deserialize,
	std::{
		net::SocketAddr,
		path::{Path, PathBuf},
	},
};

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct TracingConfig
{
	pub include_http_headers: bool,
	pub stderr: StderrConfig,
	pub files: FilesConfig,
	pub console: ConsoleConfig,

	#[cfg(target_os = "linux")]
	pub journald: JournaldConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct StderrConfig
{
	pub enable: bool,
	pub ansi: bool,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct FilesConfig
{
	pub enable: bool,

	#[serde(default = "default_files_directory")]
	pub directory: Box<Path>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct ConsoleConfig
{
	pub enable: bool,
	pub server_addr: Option<SocketAddr>,
}

#[cfg(target_os = "linux")]
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct JournaldConfig
{
	pub enable: bool,
}

impl Default for FilesConfig
{
	fn default() -> Self
	{
		Self { enable: Default::default(), directory: default_files_directory() }
	}
}

fn default_files_directory() -> Box<Path>
{
	PathBuf::from("/var/log/cs2kz-api").into_boxed_path()
}
