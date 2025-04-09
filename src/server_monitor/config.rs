use {
	serde::{Deserialize, Deserializer},
	std::time::Duration,
};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config
{
	#[serde(default = "default_timeout", deserialize_with = "deserialize_duration")]
	pub handshake_timeout: Duration,

	#[serde(default = "default_timeout", deserialize_with = "deserialize_duration")]
	pub heartbeat_interval: Duration,
}

impl Default for Config
{
	fn default() -> Self
	{
		Self { handshake_timeout: default_timeout(), heartbeat_interval: default_timeout() }
	}
}

fn default_timeout() -> Duration
{
	Duration::from_secs(30)
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
	D: Deserializer<'de>,
{
	f64::deserialize(deserializer).map(Duration::from_secs_f64)
}
