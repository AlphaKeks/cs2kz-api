//! API configuration.

use std::borrow::Cow;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZero;
use std::path::Path;
use std::time::Duration;
use std::{env, fmt, fs, io, thread};

use cookie::{Cookie, CookieBuilder};
use url::Url;

/// The global configuration for the API.
///
/// This is loaded from a TOML file on startup.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct Config {
	#[serde(default)]
	pub runtime: RuntimeConfig,

	#[serde(default)]
	pub http: HttpConfig,

	#[serde(default)]
	pub tracing: TracingConfig,

	#[serde(default)]
	pub database: DatabaseConfig,

	pub credentials: Credentials,
}

#[derive(Default, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct RuntimeConfig {
	/// The amount of worker threads to spin up.
	#[serde(default)]
	pub worker_threads: Option<NonZero<usize>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct HttpConfig {
	/// The IP address the server should listen on.
	#[serde(default = "HttpConfig::default_ip")]
	pub ip: IpAddr,

	/// The port the server should listen on.
	#[serde(default = "HttpConfig::default_port")]
	pub port: u16,

	/// Default values for HTTP cookies.
	#[serde(default)]
	pub cookies: CookieConfig,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct CookieConfig {
	/// Value for the [`Domain`] attribute.
	///
	/// [`Domain`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
	#[serde(default = "CookieConfig::default_domain")]
	pub domain: Box<str>,

	/// Value for the [`Max-Age`] attribute.
	///
	/// [`Max-Age`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#max-agenumber
	#[serde(
		default = "CookieConfig::default_max_age",
		deserialize_with = "CookieConfig::deserialize_max_age"
	)]
	pub max_age: Duration,
}

impl CookieConfig {
	/// Creates a [`CookieBuilder`] for an HTTP cookie with the given `name` and
	/// `value` with sensible default configuration.
	pub fn build_cookie<'a>(
		&self,
		name: impl Into<Cow<'a, str>>,
		value: impl Into<Cow<'a, str>>,
	) -> CookieBuilder<'a> {
		let max_age = self
			.max_age
			.try_into()
			.expect("Max-Age duration should be sensible");

		Cookie::build((name, value))
			.domain(String::from(&*self.domain))
			.http_only(true)
			.max_age(max_age)
			.path("/")
			.same_site(cookie::SameSite::Strict)
			.secure(true)
	}

	fn default_domain() -> Box<str> {
		Box::from(".cs2kz.org")
	}

	fn default_max_age() -> Duration {
		// 2 weeks
		Duration::from_secs(60 * 60 * 24 * 14)
	}

	fn deserialize_max_age<'de, D>(deserializer: D) -> Result<Duration, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u64 as serde::Deserialize<'de>>::deserialize(deserializer).map(Duration::from_secs)
	}
}

impl Default for CookieConfig {
	fn default() -> Self {
		Self {
			domain: Self::default_domain(),
			max_age: Self::default_max_age(),
		}
	}
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct TracingConfig {
	/// Whether to enable tracing.
	#[serde(default = "TracingConfig::default_enable")]
	pub enable: bool,

	/// Whether to include HTTP headers in logs.
	#[serde(default)]
	pub include_http_headers: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct DatabaseConfig {
	/// The URL we should connect to.
	#[serde(
		default = "DatabaseConfig::default_url",
		deserialize_with = "DatabaseConfig::deserialize_url"
	)]
	pub url: Url,

	/// The amount of connections to open immediately.
	///
	/// If this value is omitted, no connections will be opened immediately.
	#[serde(default)]
	pub min_connections: u32,

	/// The maximum amount of connections to open at once.
	///
	/// If this value is omitted, 1 connection per OS thread will be used as the
	/// limit.
	#[serde(default = "DatabaseConfig::default_max_connections")]
	pub max_connections: NonZero<u32>,
}

impl DatabaseConfig {
	fn default_url() -> Url {
		env::var("DATABASE_URL")
			.expect(
				"either `database.url` (config file) or `DATABASE_URL` (environment) must be set",
			)
			.parse::<Url>()
			.expect("`DATABASE_URL` must be a valid URL")
	}

	/// "Deserializes" [`DatabaseConfig::url`].
	///
	/// If there is no value to deserialize, we will fall back to using the
	/// `DATABASE_URL` environment variable.
	fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		if let Some(url) = <Option<Url> as serde::Deserialize<'de>>::deserialize(deserializer)? {
			return Ok(url);
		}

		env::var("DATABASE_URL")
			.map_err(|error| match error {
				env::VarError::NotPresent => de::Error::custom(
					"If you do not specify `database.url` in the configuration file, you must set \
					 the `DATABASE_URL` environment variable.",
				),
				env::VarError::NotUnicode(raw) => de::Error::custom(format_args!(
					"I don't know what you think you're doing, but setting `DATABASE_URL` to \
					 `{raw:?}` was probably not the best idea ever.",
				)),
			})?
			.parse::<Url>()
			.map_err(de::Error::custom)
	}

	/// Determines the default value of [`DatabaseConfig::max_connections`].
	///
	/// By default, we want to open 1 database connection per available CPU
	/// core, with a minimum of 1 if the core count cannot be detected for any
	/// reason.
	///
	/// This invariant is also reflected in the type system via [`NonZero`].
	fn default_max_connections() -> NonZero<u32> {
		thread::available_parallelism().map_or(NonZero::<u32>::MIN, |count| {
			count.try_into().expect("thread count should be reasonable")
		})
	}
}

impl Default for DatabaseConfig {
	fn default() -> Self {
		Self {
			url: Self::default_url(),
			min_connections: 0,
			max_connections: Self::default_max_connections(),
		}
	}
}

impl fmt::Debug for DatabaseConfig {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_struct("DatabaseConfig")
			.field("url", &self.url.as_str())
			.field("min_connections", &self.min_connections)
			.field("max_connections", &self.max_connections)
			.finish()
	}
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[expect(missing_docs)]
pub struct Credentials {
	/// The name of the credentials used by GitHub Actions to publish new
	/// versions of cs2kz-metamod.
	pub publish_plugin_version: Box<str>,
}

impl fmt::Debug for Credentials {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_struct("Credentials").finish_non_exhaustive()
	}
}

/// Errors that can occur when loading the [`Config`] from a file.
#[derive(Debug, Error)]
#[expect(missing_docs)]
pub enum LoadFromFileError {
	#[error("failed to read configuration file: {0}")]
	ReadFile(#[source] io::Error),

	#[error("failed to parse configuration file: {0}")]
	Deserialize(#[source] toml::de::Error),
}

impl Config {
	/// Loads a file into memory and parses it into a [`Config`] object.
	pub fn load_from_file(path: &Path) -> Result<Self, LoadFromFileError> {
		fs::read_to_string(path)
			.map(|file_contents| toml::from_str(&file_contents))
			.map_err(LoadFromFileError::ReadFile)?
			.map_err(LoadFromFileError::Deserialize)
	}
}

impl HttpConfig {
	/// Returns the [`SocketAddr`] that the HTTP server should listen on.
	pub fn socket_addr(&self) -> SocketAddr {
		SocketAddr::new(self.ip, self.port)
	}

	fn default_ip() -> IpAddr {
		IpAddr::V4(Ipv4Addr::LOCALHOST)
	}

	fn default_port() -> u16 {
		42069_u16
	}
}

impl Default for HttpConfig {
	fn default() -> Self {
		Self {
			ip: Self::default_ip(),
			port: Self::default_port(),
			cookies: CookieConfig::default(),
		}
	}
}

impl TracingConfig {
	fn default_enable() -> bool {
		true
	}
}

impl Default for TracingConfig {
	fn default() -> Self {
		Self {
			enable: Self::default_enable(),
			include_http_headers: Default::default(),
		}
	}
}
