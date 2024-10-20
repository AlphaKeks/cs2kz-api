use std::borrow::Cow;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use cookie::{Cookie, CookieBuilder};
use url::Url;

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct HttpConfig {
	#[serde(default = "default_listen_addr")]
	pub listen_addr: SocketAddr,

	#[serde(default = "default_public_url")]
	pub public_url: Url,

	#[serde(default)]
	pub cookies: CookieConfig,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CookieConfig {
	#[serde(default = "default_cookie_domain")]
	pub domain: Box<str>,

	#[serde(
		default = "default_cookie_max_age",
		deserialize_with = "deserialize_max_age"
	)]
	pub max_age: Duration,
}

impl CookieConfig {
	/// Creates a [`CookieBuilder`] based on this configuration.
	pub fn build_cookie(
		&self,
		name: impl Into<Cow<'static, str>>,
		value: impl Into<Cow<'static, str>>,
	) -> CookieBuilder<'static> {
		let max_age = self
			.max_age
			.try_into()
			.expect("max_age should have a reasonable length");

		Cookie::build((name, value))
			.domain(Cow::Owned(String::from(&*self.domain)))
			.path("/")
			.max_age(max_age)
			.secure(true)
	}
}

impl Default for HttpConfig {
	fn default() -> Self {
		Self {
			listen_addr: default_listen_addr(),
			public_url: default_public_url(),
			cookies: CookieConfig::default(),
		}
	}
}

impl Default for CookieConfig {
	fn default() -> Self {
		Self {
			domain: default_cookie_domain(),
			max_age: default_cookie_max_age(),
		}
	}
}

fn default_listen_addr() -> SocketAddr {
	SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 42069))
}

fn default_public_url() -> Url {
	Url::parse("https://api.cs2kz.org").expect("hard-coded URL should be valid")
}

fn default_cookie_domain() -> Box<str> {
	Box::from(".cs2kz.org")
}

fn default_cookie_max_age() -> Duration {
	// 2 weeks
	Duration::from_secs(60 * 60 * 24 * 14)
}

fn deserialize_max_age<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
	D: serde::Deserializer<'de>,
{
	<u64 as serde::Deserialize<'de>>::deserialize(deserializer).map(Duration::from_secs)
}
