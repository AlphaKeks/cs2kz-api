//! # Steam OpenID 2.0 Authentication
//!
//! This crate provides an implementation of OpenID 2.0 Authentication using Steam as the provider.

#![feature(non_exhaustive_omitted_patterns_lint)]
#![feature(substr_range)]
#![feature(unqualified_local_imports)]

#[macro_use(Debug)]
extern crate derive_more as _;

use std::{error::Error, fmt, str};

use bytes::Bytes;
use http_body::Body as HttpBody;
use http_body_util::BodyExt;
use serde::{
	Deserialize,
	Serialize,
	ser::{SerializeMap, Serializer},
};
use steam_id::SteamId;
use url::Url;

pub const LOGIN_URL: &str = "https://steamcommunity.com/openid/login";

/// Constructs a URL for OpenID 2.0 login with Steam.
///
/// Steam will redirect the user to `return_to` after the login process is complete. `userdata`
/// will be injected into this URL such that Steam's request will include a `userdata` field in its
/// query parameters.
#[tracing::instrument(
    level = "trace",
    skip(userdata),
    fields(return_to = return_to.as_str()),
    ret(Display, level = "debug"),
    err(level = "debug"),
)]
pub fn login_url<T>(mut return_to: Url, userdata: &T) -> Result<Url, serde_urlencoded::ser::Error>
where
	T: ?Sized + Serialize,
{
	{
		#[derive(Serialize)]
		struct UserData<'a, T: ?Sized>
		{
			userdata: &'a T,
		}

		let mut query = return_to.query_pairs_mut();
		let serializer = serde_urlencoded::Serializer::new(&mut query);

		(UserData { userdata }).serialize(serializer)?;
	}

	let query_string = serde_urlencoded::to_string(&{
		struct Form<'a>
		{
			return_to: &'a Url,
		}

		impl Serialize for Form<'_>
		{
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: Serializer,
			{
				let mut serializer = serializer.serialize_map(Some(6))?;
				let realm = {
					let return_to = self.return_to.as_str();
					let path_range = return_to
						.substr_range(self.return_to.path())
						.unwrap_or_else(|| panic!("`path` is derived from `return_to`"));

					&return_to[..(if path_range.start == 0 {
						return_to.len()
					} else {
						path_range.start
					})]
				};

				serializer.serialize_entry("openid.ns", "http://specs.openid.net/auth/2.0")?;
				serializer.serialize_entry("openid.mode", "checkid_setup")?;

				for key in ["openid.identity", "openid.claimed_id"] {
					serializer.serialize_entry(
						key,
						"http://specs.openid.net/auth/2.0/identifier_select",
					)?;
				}

				serializer.serialize_entry("openid.realm", realm)?;
				serializer.serialize_entry("openid.return_to", self.return_to)?;

				serializer.end()
			}
		}

		Form { return_to: &return_to }
	})?;

	Ok(format!("{LOGIN_URL}?{query_string}").parse().unwrap_or_else(|err| {
		panic!("hard-coded URL with valid query string should be a valid URL: {err}");
	}))
}

/// Payload sent by Steam after the login process is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallbackPayload
{
	#[serde(rename = "openid.ns")]
	pub ns: String,

	#[serde(rename = "openid.identity")]
	pub identity: Option<String>,

	#[debug("{:?}", claimed_id.as_str())]
	#[serde(rename = "openid.claimed_id")]
	pub claimed_id: Url,

	#[serde(rename = "openid.mode")]
	pub mode: String,

	#[debug("{:?}", return_to.as_str())]
	#[serde(rename = "openid.return_to")]
	pub return_to: Url,

	#[serde(rename = "openid.op_endpoint")]
	pub op_endpoint: String,

	#[serde(rename = "openid.response_nonce")]
	pub response_nonce: String,

	#[serde(rename = "openid.invalidate_handle")]
	pub invalidate_handle: Option<String>,

	#[serde(rename = "openid.assoc_handle")]
	pub assoc_handle: String,

	#[serde(rename = "openid.signed")]
	pub signed: String,

	#[serde(rename = "openid.sig")]
	pub sig: String,

	/// The serialized `userdata` injected by [`login_url()`].
	#[serde(skip_serializing)]
	pub userdata: String,
}

#[derive(Debug)]
pub struct VerifyCallbackPayloadError<HttpError, ResponseBody>
where
	ResponseBody: HttpBody<Error: Error + 'static>,
{
	kind: VerifyCallbackPayloadErrorKind<HttpError, ResponseBody>,
	payload: CallbackPayload,
}

#[derive(Debug)]
pub enum VerifyCallbackPayloadErrorKind<HttpError, ResponseBody>
where
	ResponseBody: HttpBody<Error: Error + 'static>,
{
	HostMismatch,
	HttpRequest(HttpError),
	BadStatus
	{
		response: http::Response<Bytes>,
	},
	BufferResponseBody
	{
		error: ResponseBody::Error,
		response: http::response::Parts,
	},
	InvalidPayload
	{
		response: http::Response<Bytes>,
	},
}

impl CallbackPayload
{
	#[tracing::instrument(skip(self, send_request), ret(level = "debug"), err(level = "debug"))]
	pub async fn verify<S, E, ResponseBody>(
		mut self,
		expected_host: url::Host<&str>,
		send_request: S,
	) -> Result<SteamId, VerifyCallbackPayloadError<E, ResponseBody>>
	where
		S: AsyncFnOnce(http::Request<Bytes>) -> Result<http::Response<ResponseBody>, E>,
		ResponseBody: HttpBody<Error: Error + 'static>,
	{
		if self.return_to.host() != Some(expected_host) {
			return Err(VerifyCallbackPayloadError {
				kind: VerifyCallbackPayloadErrorKind::HostMismatch,
				payload: self,
			});
		}

		if self.mode != "check_authentication" {
			self.mode.clear();
			self.mode.push_str("check_authentication");
		}

		let payload = serde_urlencoded::to_string(&self).unwrap_or_else(|err| {
			panic!("`CallbackPayload` should always serialize properly: {err}")
		});

		let request = http::Request::post(LOGIN_URL)
			.header(http::header::CONTENT_TYPE, mime::APPLICATION_WWW_FORM_URLENCODED.as_ref())
			.header(http::header::ORIGIN, "https://steamcommunity.com")
			.header(http::header::REFERER, "https://steamcommunity.com/")
			.body(Bytes::from(payload))
			.unwrap_or_else(|err| panic!("hard-coded HTTP request should be valid: {err}"));

		let (response, body) = match send_request(request).await {
			Ok(response) => response.into_parts(),
			Err(err) => {
				return Err(VerifyCallbackPayloadError {
					kind: VerifyCallbackPayloadErrorKind::HttpRequest(err),
					payload: self,
				});
			},
		};

		let body = match body.collect().await {
			Ok(collected) => collected.to_bytes(),
			Err(error) => {
				return Err(VerifyCallbackPayloadError {
					kind: VerifyCallbackPayloadErrorKind::BufferResponseBody { error, response },
					payload: self,
				});
			},
		};

		if !response.status.is_success() {
			return Err(VerifyCallbackPayloadError {
				kind: VerifyCallbackPayloadErrorKind::BadStatus {
					response: http::Response::from_parts(response, body),
				},
				payload: self,
			});
		}

		if !body[..].rsplit(|&byte| byte == b'\n').any(|line| line == b"is_valid:true") {
			return Err(VerifyCallbackPayloadError {
				kind: VerifyCallbackPayloadErrorKind::InvalidPayload {
					response: http::Response::from_parts(response, body),
				},
				payload: self,
			});
		}

		Ok(self
			.claimed_id
			.path_segments()
			.and_then(Iterator::last)
			.and_then(|segment| segment.parse::<SteamId>().ok())
			.unwrap_or_else(|| panic!("Steam should return a valid SteamID")))
	}
}

impl<HttpError, ResponseBody> VerifyCallbackPayloadError<HttpError, ResponseBody>
where
	ResponseBody: HttpBody<Error: Error + 'static>,
{
	pub fn kind(&self) -> &VerifyCallbackPayloadErrorKind<HttpError, ResponseBody>
	{
		&self.kind
	}

	pub fn into_payload(self) -> CallbackPayload
	{
		self.payload
	}
}

impl<HttpError, ResponseBody> fmt::Display for VerifyCallbackPayloadError<HttpError, ResponseBody>
where
	ResponseBody: HttpBody<Error: Error + 'static>,
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.write_str("failed to verify callback payload: ")?;

		match self.kind {
			VerifyCallbackPayloadErrorKind::HostMismatch => {
				fmt.write_str("`return_to` host does not match our host")
			},
			VerifyCallbackPayloadErrorKind::HttpRequest(_) => {
				fmt.write_str("failed to make HTTP request to Steam")
			},
			VerifyCallbackPayloadErrorKind::BadStatus { ref response } => {
				write!(fmt, "HTTP request returned a bad status code ({})", response.status())
			},
			VerifyCallbackPayloadErrorKind::BufferResponseBody { .. } => {
				fmt.write_str("failed to buffer response body")
			},
			VerifyCallbackPayloadErrorKind::InvalidPayload { .. } => {
				fmt.write_str("invalid payload")
			},
		}
	}
}

impl<HttpError, ResponseBody> Error for VerifyCallbackPayloadError<HttpError, ResponseBody>
where
	HttpError: Error + 'static,
	ResponseBody: HttpBody<Error: Error + 'static>,
{
	fn source(&self) -> Option<&(dyn Error + 'static)>
	{
		match self.kind {
			VerifyCallbackPayloadErrorKind::HostMismatch
			| VerifyCallbackPayloadErrorKind::BadStatus { .. }
			| VerifyCallbackPayloadErrorKind::InvalidPayload { .. } => None,
			VerifyCallbackPayloadErrorKind::HttpRequest(ref error) => Some(error),
			VerifyCallbackPayloadErrorKind::BufferResponseBody { ref error, .. } => Some(error),
		}
	}
}
