//! This module holds configuration for the API.
//!
//! This configuration is passed to [`run()`], which acts as the main entry point for the API.
//! The [`Config`] can be read from a file, or constructed in any other way you'd like.
//!
//! [`run()`]: crate::run

use serde::Deserialize;

pub mod http;
pub mod database;
pub mod steam;
pub mod tracing;
pub mod runtime;

/// Top-level configuration for the API.
///
/// See the `.config/cs2kz-api.example.toml` file in the root of the repository for detailed
/// documentation and example values for all of the options.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[expect(missing_docs)]
pub struct Config
{
	pub http: http::Config,

	#[serde(default)]
	pub database: database::Config,

	pub steam: steam::Config,

	#[serde(default)]
	pub tracing: tracing::Config,

	#[serde(default)]
	pub runtime: runtime::Config,
}
