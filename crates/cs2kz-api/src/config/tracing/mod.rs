use serde::{de, Deserialize, Deserializer};
use tracing_subscriber::EnvFilter;

pub mod stderr;
pub mod files;
pub mod console;

#[cfg(target_os = "linux")]
pub mod journald;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Initialize a tracing subscriber.
	#[serde(default)]
	pub enable: bool,

	/// Global filters that apply to all layers.
	#[serde(default)]
	pub filters: Vec<Filter>,

	/// Configuration for the layer emitting to stderr.
	pub stderr: Option<stderr::Config>,

	/// Configuration for the layer emitting to files.
	pub files: Option<files::Config>,

	/// Configuration for the layer emitting to tokio-console.
	pub console: Option<console::Config>,

	/// Configuration for the layer emitting to systemd's journald.
	#[cfg(target_os = "linux")]
	pub journald: Option<journald::Config>,
}

impl Config
{
	pub fn env_filter(&self) -> Option<EnvFilter>
	{
		(!self.filters.is_empty()).then(|| {
			self.filters
				.iter()
				.map(|filter| filter.directive.clone())
				.fold(EnvFilter::from_default_env(), EnvFilter::add_directive)
		})
	}
}

#[derive(Debug)]
pub struct Filter
{
	pub directive: tracing_subscriber::filter::Directive,
}

impl<'de> Deserialize<'de> for Filter
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		String::deserialize(deserializer)?
			.parse()
			.map(|directive| Self { directive })
			.map_err(de::Error::custom)
	}
}
