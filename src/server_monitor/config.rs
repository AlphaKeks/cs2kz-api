use {
	crate::time::DurationExt,
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

	#[serde(
		default = "default_inactivity_check_interval",
		deserialize_with = "deserialize_duration"
	)]
	pub inactivity_check_interval: Duration,

	#[serde(
		default = "default_inactivity_check_threshold",
		deserialize_with = "deserialize_duration"
	)]
	pub inactivity_check_threshold: Duration,
}

impl Default for Config
{
	fn default() -> Self
	{
		Self {
			handshake_timeout: default_timeout(),
			heartbeat_interval: default_timeout(),
			inactivity_check_interval: default_inactivity_check_interval(),
			inactivity_check_threshold: default_inactivity_check_threshold(),
		}
	}
}

fn default_timeout() -> Duration
{
	Duration::from_secs(30)
}

fn default_inactivity_check_interval() -> Duration
{
	Duration::HOUR * 6
}

fn default_inactivity_check_threshold() -> Duration
{
	Duration::DAY * 3
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
	D: Deserializer<'de>,
{
	f64::deserialize(deserializer).map(Duration::from_secs_f64)
}
