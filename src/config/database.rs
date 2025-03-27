use std::num::NonZero;

use cs2kz_api::database::ConnectOptions;
use serde::{Deserialize, Deserializer};
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct DatabaseConfig
{
	#[serde(default = "default_url")]
	pub url: Url,

	#[serde(default, deserialize_with = "deserialize_option_u32")]
	pub min_connections: Option<NonZero<u32>>,

	#[serde(default, deserialize_with = "deserialize_option_u32")]
	pub max_connections: Option<NonZero<u32>>,
}

impl DatabaseConfig
{
	pub(crate) fn connect_options(&self) -> ConnectOptions<'_>
	{
		ConnectOptions::builder()
			.url(&self.url)
			.maybe_min_connections(self.min_connections)
			.maybe_max_connections(self.max_connections)
			.build()
	}
}

impl Default for DatabaseConfig
{
	fn default() -> Self
	{
		Self {
			url: default_url(),
			min_connections: None,
			max_connections: None,
		}
	}
}

fn default_url() -> Url
{
	Url::parse("mysql://schnose:very-secure-password@localhost/kz").unwrap_or_else(|err| {
		panic!("hard-coded URL should be valid\n{err}");
	})
}

fn deserialize_option_u32<'de, D>(deserializer: D) -> Result<Option<NonZero<u32>>, D::Error>
where
	D: Deserializer<'de>,
{
	<Option<u32> as Deserialize<'de>>::deserialize(deserializer)
		.map(|maybe_num| maybe_num.and_then(NonZero::new))
}
