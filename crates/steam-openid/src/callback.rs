//! This module contains the [`CallbackPayload`] and related types.

use std::future;

use serde::{Deserialize, Serialize};
use tower_service::Service;
use url::Url;

/// Payload sent by Steam after the login process is complete.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(missing_docs, reason = "should be self-explanatory")]
pub struct CallbackPayload {
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
	///
	/// [`LoginForm::redirect_url()`]: crate::LoginForm::redirect_url()
	#[serde(skip_serializing)]
	pub userdata: String,
}

impl CallbackPayload {
	/// Verifies the payload with Steam.
	///
	/// The `expected_realm` parameter should match the [`realm`] you passed to
	/// [`LoginForm::new()`].
	///
	/// [`realm`]: crate::LoginForm::realm
	/// [`LoginForm::new()`]: crate::LoginForm::new()
	#[expect(
		clippy::future_not_send,
		reason = "callers control `Send`ness, and we don't want to be opinionated"
	)]
	pub async fn verify<S, ReqBody, ResBody>(
		&mut self,
		expected_realm: &Url,
		mut http_client: S,
	) -> Result<(), VerifyCallbackPayloadError<S, ReqBody>>
	where
		S: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>,
		S::Error: std::error::Error + 'static,
		ReqBody: From<String>,
		ResBody: AsRef<[u8]>,
	{
		/// Value to use for the `Content-Type` header in our request.
		const POST_CONTENT_TYPE: http::HeaderValue =
			http::HeaderValue::from_static("application/x-www-form-urlencoded");

		if self.return_to.host() != expected_realm.host() {
			return Err(VerifyCallbackPayloadError::RealmMismatch);
		}

		self.mode.clear();
		self.mode.push_str("check_authentication");

		let payload = serde_urlencoded::to_string(self)
			.map(ReqBody::from)
			.expect("if payload could be deserialized it should be re-serializable");

		let request = http::Request::post(crate::LOGIN_URL)
			.header(http::header::CONTENT_TYPE, POST_CONTENT_TYPE)
			.body(payload)
			.expect("valid http request");

		future::poll_fn(|cx| http_client.poll_ready(cx))
			.await
			.map_err(VerifyCallbackPayloadError::HttpClient)?;

		let response = http_client
			.call(request)
			.await
			.map_err(VerifyCallbackPayloadError::HttpClient)?;

		let response_body = std::str::from_utf8(response.body().as_ref())
			.expect("Steam always sends UTF-8 text here");

		if response_body
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
	/// This may return [`None`] on invalid payloads, but if you [verify] the
	/// payload first, it _should_ always succeed.
	///
	/// [verify]: CallbackPayload::verify()
	pub fn steam_id(&self) -> Option<u64> {
		self.claimed_id
			.path_segments()
			.and_then(Iterator::last)
			.and_then(|segment| segment.parse::<u64>().ok())
	}
}

/// Errors that could occur when verifying a [`CallbackPayload`].
#[derive(Debug, Error)]
pub enum VerifyCallbackPayloadError<S, ReqBody>
where
	S: Service<http::Request<ReqBody>>,
	S::Error: std::error::Error + 'static,
{
	/// The realm included in the query did not match the expected realm.
	#[error("realm mismatch")]
	RealmMismatch,

	/// The HTTP client returned an error.
	#[error("http error")]
	HttpClient(#[source] S::Error),

	/// The payload was not deemed valid by Steam.
	#[error("payload is invalid")]
	InvalidPayload,
}
