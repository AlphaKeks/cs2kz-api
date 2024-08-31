use std::net::SocketAddr;

use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Emit traces to files.
	#[serde(default)]
	pub enable: bool,

	/// Address the console server should listen on.
	#[serde(
		default = "default_server_addr",
		deserialize_with = "deserialize_server_addr"
	)]
	pub server_addr: console_subscriber::ServerAddr,
}

fn default_server_addr() -> console_subscriber::ServerAddr
{
	console_subscriber::ServerAddr::Tcp(SocketAddr::new(
		console_subscriber::Server::DEFAULT_IP,
		console_subscriber::Server::DEFAULT_PORT,
	))
}

fn deserialize_server_addr<'de, D>(
	deserializer: D,
) -> Result<console_subscriber::ServerAddr, D::Error>
where
	D: Deserializer<'de>,
{
	String::deserialize(deserializer).map(|value| match value.parse::<SocketAddr>() {
		Ok(addr) => console_subscriber::ServerAddr::Tcp(addr),
		Err(_) => console_subscriber::ServerAddr::Unix(value.into()),
	})
}
