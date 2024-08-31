use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use super::Filter;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Emit traces to stderr.
	#[serde(default)]
	pub enable: bool,

	/// Emit ANSI escape codes for colors and other formatting.
	#[serde(default)]
	pub ansi: bool,

	/// Filters that apply just to this layer.
	#[serde(default)]
	pub filters: Vec<Filter>,
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
