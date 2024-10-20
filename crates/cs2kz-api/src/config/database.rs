use std::num::NonZero;
use std::{env, thread};

use serde::de;
use url::Url;

/// Database configuration.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DatabaseConfig {
	/// The URL we should connect to.
	#[debug("\"{url}\"")]
	#[serde(default = "default_url", deserialize_with = "deserialize_url")]
	pub url: Url,

	/// The amount of connections to open immediately.
	///
	/// If this value is omitted, no connections will be opened immediately.
	#[serde(default)]
	pub min_connections: u32,

	/// The maximum amount of connections to open at once.
	///
	/// If this value is omitted, 1 connection per OS thread will be used as the limit.
	#[serde(default = "default_max_connections")]
	pub max_connections: NonZero<u32>,
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self {
			url: default_url(),
			min_connections: 0,
			max_connections: default_max_connections(),
		}
	}
}

fn default_url() -> Url {
	env::var("DATABASE_URL")
		.expect("either `database.url` (config file) or `DATABASE_URL` (environment) must be set")
		.parse::<Url>()
		.expect("`DATABASE_URL` must be a valid URL")
}

/// "Deserializes" [`DatabaseConfig::url`].
///
/// If there is no value to deserialize, we will fall back to using the `DATABASE_URL` environment
/// variable.
fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
where
	D: serde::Deserializer<'de>,
{
	if let Some(url) = <Option<Url> as serde::Deserialize<'de>>::deserialize(deserializer)? {
		return Ok(url);
	}

	env::var("DATABASE_URL")
		.map_err(|error| match error {
			env::VarError::NotPresent => de::Error::custom(
				"If you do not specify `database.url` in the configuration file, \
				 you must set the `DATABASE_URL` environment variable.",
			),
			env::VarError::NotUnicode(raw) => de::Error::custom(format_args!(
				"I don't know what you think you're doing, but setting \
				 `DATABASE_URL` to `{raw:?}` was probably not the best idea ever.",
			)),
		})?
		.parse::<Url>()
		.map_err(de::Error::custom)
}

/// Determines the default value of [`DatabaseConfig::max_connections`].
///
/// By default, we want to open 1 database connection per available CPU core, with a minimum of
/// 1 if the core count cannot be detected for any reason.
///
/// This invariant is also reflected in the type system via [`NonZero`].
fn default_max_connections() -> NonZero<u32> {
	thread::available_parallelism().map_or(NonZero::<u32>::MIN, |count| {
		count.try_into().expect("thread count should be reasonable")
	})
}
