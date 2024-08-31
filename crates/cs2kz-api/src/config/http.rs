use std::net::SocketAddr;

use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// The address the HTTP server is supposed to listen on.
	pub listen_on: SocketAddr,

	/// The URL other services are supposed to use to reach the API.
	pub public_url: Url,

	/// The value to use for the `Domain` field in HTTP cookies.
	pub cookie_domain: Box<str>,
}
