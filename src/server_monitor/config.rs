use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct ServerMonitorConfig
{
	#[serde(default = "default_timeout")]
	pub handshake_timeout: Duration,

	#[serde(default = "default_timeout")]
	pub heartbeat_interval: Duration,
}

impl Default for ServerMonitorConfig
{
	fn default() -> Self
	{
		Self {
			handshake_timeout: default_timeout(),
			heartbeat_interval: default_timeout(),
		}
	}
}

fn default_timeout() -> Duration
{
	Duration::from_secs(30)
}
