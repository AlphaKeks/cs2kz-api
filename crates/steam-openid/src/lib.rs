//! # Steam OpenID authentication
//!
//! Steam can act as an OpenID 2.0 authentication provider.
//! This crate provides types and functions to perform this flow.
//!
//! ## Usage
//!
//! 1. Create a [`LoginForm`].
//! 2. Turn the form into a URL using [`LoginForm::redirect_url()`] and redirect your user there.
//! 3. In your callback endpoint, deserialize the request query parameters into
//!    a [`CallbackPayload`].
//! 5. Verify the payload by calling [`CallackPayload::verify()`].
//! 6. Extract any information you need. You can get the user's SteamID by calling
//!    [`CallbackPayload::steam_id()`].

use std::future;
use std::num::NonZero;

use serde::{Deserialize, Serialize};
use tower_service::Service;
use url::Url;

mod errors;
pub use errors::{CreateRedirectUrlError, VerifyCallbackPayloadError};

/// Steam URL to redirect the user in for login.
pub const LOGIN_URL: &str = "https://steamcommunity.com/openid/login";

/// Query parameters that will be included in the initial redirect to Steam.
#[derive(Debug, Clone, Serialize)]
#[expect(missing_docs, reason = "these should be self-explanatory")]
pub struct LoginForm
{
	#[serde(rename = "openid.ns")]
	pub namespace: &'static str,

	#[serde(rename = "openid.identity")]
	pub identity: &'static str,

	#[serde(rename = "openid.claimed_id")]
	pub claimed_id: &'static str,

	#[serde(rename = "openid.mode")]
	pub mode: &'static str,

	#[serde(rename = "openid.realm")]
	pub realm: Url,

	#[serde(rename = "openid.return_to")]
	pub return_to: Url,
}

impl LoginForm
{
	/// Creates a new [`LoginForm`].
	///
	/// The `realm` parameter should be the base URL Steam should send its response to.
	/// `callback_route` will be appended to the base URL, and the result is the URL Steam will
	/// send its response to.
	///
	/// # Panics
	///
	/// This function will panic if `realm` and `callback_route` can't be joined into a URL.
	pub fn new(realm: Url, callback_route: &str) -> Self
	{
		let return_to = realm
			.join(callback_route)
			.expect("`realm` + `callback_route` should produce a valid URL");

		Self {
			namespace: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			realm,
			return_to,
		}
	}

	/// Constructs a URL you can redirect your users to for the login process.
	///
	/// # Errors
	///
	/// This function will return an error if `userdata` cannot be serialized as a URL query
	/// parameter.
	pub fn redirect_url<T>(mut self, userdata: &T) -> Result<Url, CreateRedirectUrlError>
	where
		T: Serialize,
	{
		{
			let mut query_pairs = self.return_to.query_pairs_mut();
			let serializer = serde_urlencoded::Serializer::new(&mut query_pairs);

			userdata.serialize(serializer)?;
		}

		let query_string = serde_urlencoded::to_string(&self)?;
		let mut url = Url::parse(LOGIN_URL).expect("valid url");
		url.set_query(Some(&query_string));

		Ok(url)
	}
}

/// Payload included as query parameters when Steam redirects the user back to your callback
/// endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(missing_docs, reason = "these should be self-explanatory")]
pub struct CallbackPayload
{
	#[serde(rename = "openid.ns")]
	pub namespace: String,

	#[serde(rename = "openid.identity")]
	pub identity: Option<String>,

	#[serde(rename = "openid.claimed_id")]
	pub claimed_id: Url,

	#[serde(rename = "openid.mode")]
	pub mode: String,

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

	/// The injected userdata that was passed as an argument to
	/// [`LoginForm::redirect_url()`].
	#[serde(skip_serializing)]
	pub userdata: String,
}

impl CallbackPayload
{
	/// Verifies the payload by sending it back to Steam.
	///
	/// The `expected_realm` parameter is used to verify that the request Steam is making was
	/// actually initiated by you, so it should have the same value as the `realm` parameter
	/// you passed to [`LoginForm::new()`].
	pub async fn verify<S, ReqBody, ResBody>(
		&mut self,
		expected_realm: &Url,
		mut http_client: S,
	) -> Result<(), VerifyCallbackPayloadError<S, http::Request<ReqBody>>>
	where
		S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Send,
		S::Error: std::error::Error + 'static,
		S::Future: Send,
		ReqBody: From<String> + Send,
		ResBody: AsRef<[u8]>,
	{
		const CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

		if self.return_to.host() != expected_realm.host() {
			return Err(VerifyCallbackPayloadError::InvalidPayload);
		}

		self.mode.clear();
		self.mode.push_str("check_authentication");

		let body = serde_urlencoded::to_string(self)
			.map(ReqBody::from)
			.expect("if payload could be deserialized, it should be re-serializable");

		let request = http::Request::post(LOGIN_URL)
			.header(http::header::CONTENT_TYPE, CONTENT_TYPE)
			.body(body)
			.expect("valid http request");

		future::poll_fn(|cx| http_client.poll_ready(cx))
			.await
			.map_err(VerifyCallbackPayloadError::HttpClient)?;

		let response = http_client
			.call(request)
			.await
			.map_err(VerifyCallbackPayloadError::HttpClient)?;

		let body = std::str::from_utf8(response.body().as_ref())
			.map_err(VerifyCallbackPayloadError::ResponseBodyNotUtf8)?;

		if body
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_none()
		{
			return Err(VerifyCallbackPayloadError::InvalidPayload);
		}

		Ok(())
	}

	/// Returns the user's SteamID.
	///
	/// This may return [`None`] on invalid payloads, but if you [verify] the payload first, it
	/// _should_ always succeed.
	///
	/// [verify]: CallbackPayload::verify()
	pub fn steam_id(&self) -> Option<NonZero<u64>>
	{
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|segment| segment.parse::<NonZero<u64>>().ok())
	}
}
