use {
	crate::runtime,
	cookie::{CookieBuilder, SameSite},
	cs2kz_api::time::DurationExt,
	serde::{Deserialize, Deserializer, de},
	std::{
		borrow::Cow,
		fmt::{self, Write},
		net::{IpAddr, Ipv4Addr, SocketAddr},
		time::Duration,
	},
	url::Url,
};

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct HttpConfig
{
	#[serde(default = "default_ip_addr")]
	pub ip_addr: IpAddr,

	#[serde(default = "default_port")]
	pub port: u16,

	#[debug("{:?}", public_url.as_str())]
	pub public_url: Url,

	#[serde(default = "default_handler_timeout", deserialize_with = "deserialize_duration")]
	pub handler_timeout: Duration,

	#[serde(default = "default_shutdown_timeout", deserialize_with = "deserialize_duration")]
	pub shutdown_timeout: Duration,

	#[serde(default = "default_session_duration", deserialize_with = "deserialize_duration")]
	pub session_duration: Duration,
	pub cors: CorsConfig,
	pub cookies: CookieConfig,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct CorsConfig
{
	#[serde(default = "default_allowed_origins", deserialize_with = "deserialize_allowed_origins")]
	pub allowed_origins: Box<[http::HeaderValue]>,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct CookieConfig
{
	/// The default value for the [`Domain`] field
	///
	/// [`Domain`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#domaindomain-value
	#[serde(default = "default_domain")]
	pub domain: Box<str>,

	/// The default value for the [`Max-Age`] field (in seconds)
	///
	/// [`Max-Age`]: https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Set-Cookie#max-agenumber
	#[debug("{max_age}")]
	#[serde(default = "default_max_age", deserialize_with = "deserialize_max_age")]
	pub max_age: time::Duration,

	/// Same as the [`max_age`] field, but for authentication cookies
	///
	/// [`max_age`]: Cookies::max_age
	#[debug("{max_age_auth}")]
	#[serde(default = "default_max_age_auth", deserialize_with = "deserialize_max_age")]
	pub max_age_auth: time::Duration,
}

impl HttpConfig
{
	pub(crate) fn socket_addr(&self) -> SocketAddr
	{
		SocketAddr::new(self.ip_addr, self.port)
	}
}

impl Default for HttpConfig
{
	fn default() -> Self
	{
		let ip_addr = default_ip_addr();
		let port = default_port();
		let public_url = {
			let mut url = String::from("http://");

			if ip_addr.is_ipv6() {
				let _ = write!(url, "[");
			}

			let _ = write!(url, "{ip_addr}");

			if ip_addr.is_ipv6() {
				let _ = write!(url, "]");
			}

			let _ = write!(url, ":{port}");

			url.parse::<Url>()
				.unwrap_or_else(|err| panic!("failed to parse hard-coded URL: {err}"))
		};
		let handler_timeout = default_handler_timeout();
		let shutdown_timeout = default_shutdown_timeout();
		let session_duration = default_session_duration();
		let cors = CorsConfig::default();
		let cookies = CookieConfig::default();

		Self {
			ip_addr,
			port,
			public_url,
			handler_timeout,
			shutdown_timeout,
			session_duration,
			cors,
			cookies,
		}
	}
}

impl CorsConfig
{
	pub(crate) fn allowed_origins(&self) -> impl Iterator<Item = http::HeaderValue>
	{
		self.allowed_origins.iter().cloned()
	}
}

impl Default for CorsConfig
{
	fn default() -> Self
	{
		Self { allowed_origins: default_allowed_origins() }
	}
}
impl CookieConfig
{
	pub(crate) fn cookie_builder<'a>(
		&self,
		name: impl Into<Cow<'a, str>>,
		value: impl Into<Cow<'a, str>>,
	) -> CookieBuilder<'a>
	{
		CookieBuilder::new(name, value)
			.domain(self.domain.to_string())
			.http_only(false)
			.max_age(self.max_age)
			.path("/")
			.same_site(SameSite::Lax)
			.secure(!runtime::environment::get().is_development())
	}

	pub(crate) fn auth_cookie_builder<'a>(
		&self,
		name: impl Into<Cow<'a, str>>,
		value: impl Into<Cow<'a, str>>,
	) -> CookieBuilder<'a>
	{
		self.cookie_builder(name, value).http_only(true).max_age(self.max_age_auth)
	}
}

impl Default for CookieConfig
{
	fn default() -> Self
	{
		Self {
			domain: default_domain(),
			max_age: default_max_age(),
			max_age_auth: default_max_age_auth(),
		}
	}
}

fn default_ip_addr() -> IpAddr
{
	IpAddr::V4(Ipv4Addr::LOCALHOST)
}

fn default_port() -> u16
{
	0
}

fn default_handler_timeout() -> Duration
{
	Duration::from_secs(90)
}

fn default_shutdown_timeout() -> Duration
{
	Duration::from_secs(10)
}

fn default_session_duration() -> Duration
{
	Duration::WEEK * 2
}

fn default_allowed_origins() -> Box<[http::HeaderValue]>
{
	Box::from([
		http::HeaderValue::from_static("https://cs2kz.org"),
		http::HeaderValue::from_static("https://dashboard.cs2kz.org"),
		http::HeaderValue::from_static("https://docs.cs2kz.org"),
		http::HeaderValue::from_static("https://forum.cs2kz.org"),
	])
}

fn default_domain() -> Box<str>
{
	Box::from(".cs2kz.org")
}

fn default_max_age() -> time::Duration
{
	time::Duration::MONTH * 3
}

fn default_max_age_auth() -> time::Duration
{
	time::Duration::WEEK
}

fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
	D: Deserializer<'de>,
{
	f64::deserialize(deserializer).map(Duration::from_secs_f64)
}

fn deserialize_allowed_origins<'de, D>(
	deserializer: D,
) -> Result<Box<[http::HeaderValue]>, D::Error>
where
	D: Deserializer<'de>,
{
	struct HeaderListVisitor;

	impl<'de> de::Visitor<'de> for HeaderListVisitor
	{
		type Value = Box<[http::HeaderValue]>;

		fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
		{
			fmt.write_str("a list of CORS origins")
		}

		fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
		where
			A: de::SeqAccess<'de>,
		{
			let size_hint = seq.size_hint().unwrap_or_default();
			let mut header_values = Vec::with_capacity(size_hint);

			while let Some(origin) = seq.next_element::<Url>()? {
				match http::HeaderValue::from_str(origin.as_str()) {
					Ok(header_value) => header_values.push(header_value),
					Err(err) => {
						return Err(de::Error::custom(format_args!("invalid CORS origin: {err}")));
					},
				}
			}

			Ok(header_values.into_boxed_slice())
		}
	}

	deserializer.deserialize_seq(HeaderListVisitor)
}

fn deserialize_max_age<'de, D>(deserializer: D) -> Result<time::Duration, D::Error>
where
	D: serde::Deserializer<'de>,
{
	i64::deserialize(deserializer).map(time::Duration::seconds)
}
