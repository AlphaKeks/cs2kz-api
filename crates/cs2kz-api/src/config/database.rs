//! Database configuration.

use std::num::NonZero;

use serde::Deserialize;

/// Database configuration.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// The minimum number of pool connections to open.
	#[serde(default)]
	pub min_connections: u32,

	/// The maximum number of pool connections to open.
	#[serde(
		default,
		deserialize_with = "crate::util::num::deserialize_non_zero_u32_opt"
	)]
	pub max_connections: Option<NonZero<u32>>,
}
