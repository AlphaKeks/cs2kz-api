mod runtime;
pub use runtime::RuntimeConfig;

mod database;
pub use database::DatabaseConfig;

mod http;
pub use http::{CookieConfig, HttpConfig};

mod tracing;
pub use tracing::TracingConfig;

mod steam;
pub use steam::SteamConfig;

/// The API's global configuration.
///
/// This struct is deserialized from a TOML file on startup, and some of the options can be
/// overridden by CLI arguments.
#[derive(Default, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ApiConfig {
	#[serde(default)]
	pub runtime: RuntimeConfig,

	#[serde(default)]
	pub database: DatabaseConfig,

	#[serde(default)]
	pub http: HttpConfig,

	#[serde(default)]
	pub tracing: TracingConfig,

	#[serde(default)]
	pub steam: SteamConfig,
}
