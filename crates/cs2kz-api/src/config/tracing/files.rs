//! Tracing configuration for emitting logs to files.

use std::path::Path;

use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use super::Filter;

/// Tracing configuration for emitting logs to files.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Emit traces to files.
	#[serde(default)]
	pub enable: bool,

	/// Directory to store log files in.
	pub directory: Box<Path>,

	/// Filters that apply just to this layer.
	#[serde(default)]
	pub filters: Vec<Filter>,
}

impl Config
{
	/// Constructs an [`EnvFilter`] from the filter directives specified in the config.
	pub fn env_filter(&self) -> Option<EnvFilter>
	{
		(!self.filters.is_empty()).then(|| {
			self.filters
				.iter()
				.map(|Filter(filter)| filter.clone())
				.fold(EnvFilter::from_default_env(), EnvFilter::add_directive)
		})
	}
}
