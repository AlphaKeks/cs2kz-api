use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::request;
use axum_extra::extract::Query;
use cs2kz::SteamID;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use url::Url;

use super::{Error, Result};

/// Form parameters that will be sent to Steam when redirecting a user for
/// login.
#[derive(Debug, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct LoginForm
{
	#[serde(rename = "openid.ns")]
	namespace: &'static str,

	#[serde(rename = "openid.identity")]
	identity: &'static str,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: &'static str,

	#[serde(rename = "openid.mode")]
	mode: &'static str,

	#[serde(rename = "openid.realm")]
	realm: Url,

	#[serde(rename = "openid.return_to")]
	return_to: Url,
}

impl LoginForm
{
	/// The API route that Steam should redirect back to after a successful
	/// login.
	pub const RETURN_ROUTE: &'static str = "/auth/callback";

	/// Steam URL to redirect the user in for login.
	pub const LOGIN_URL: &'static str = "https://steamcommunity.com/openid/login";

	/// Creates a new [`LoginForm`].
	///
	/// `realm` is the base URL of the API.
	pub(super) fn new(realm: Url) -> Self
	{
		let return_to = realm.join(Self::RETURN_ROUTE).expect("this is valid");

		Self {
			namespace: "http://specs.openid.net/auth/2.0",
			identity: "http://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "http://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			realm,
			return_to,
		}
	}

	/// Generates an OpenID URL that can be used for logging in with Steam.
	pub fn redirect_to(mut self, redirect_to: &Url) -> Url
	{
		self.return_to
			.query_pairs_mut()
			.append_pair("redirect_to", redirect_to.as_str());

		let query_string =
			serde_urlencoded::to_string(&self).expect("this is a valid query string");

		let mut url = Url::parse(Self::LOGIN_URL).expect("this is a valid url");

		url.set_query(Some(&query_string));
		url
	}
}

/// Form parameters that Steam will send to us after a successful login.
///
/// These can be sent back to Steam for validation, see
/// [`OpenIDPayload::verify()`].
#[derive(Debug, Serialize, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct OpenIDPayload
{
	/// The injected query parameter that was passed as an argument to
	/// [`LoginForm::redirect_to()`].
	#[serde(skip_serializing)]
	pub redirect_to: Url,

	#[serde(rename = "openid.ns")]
	namespace: String,

	#[serde(rename = "openid.identity")]
	identity: Option<String>,

	#[serde(rename = "openid.claimed_id")]
	claimed_id: Url,

	#[serde(rename = "openid.mode")]
	mode: String,

	#[serde(rename = "openid.return_to")]
	return_to: Url,

	#[serde(rename = "openid.op_endpoint")]
	op_endpoint: String,

	#[serde(rename = "openid.response_nonce")]
	response_nonce: String,

	#[serde(rename = "openid.invalidate_handle")]
	invalidate_handle: Option<String>,

	#[serde(rename = "openid.assoc_handle")]
	assoc_handle: String,

	#[serde(rename = "openid.signed")]
	signed: String,

	#[serde(rename = "openid.sig")]
	sig: String,
}

impl OpenIDPayload
{
	/// Verifies this payload with Steam and extracts the user's SteamID from
	/// it.
	#[tracing::instrument(level = "debug", skip_all, ret, fields(
		redirect_to = %self.redirect_to
	))]
	async fn verify(mut self, http_client: &reqwest::Client) -> Result<Self>
	{
		self.mode = String::from("check_authentication");

		let response = http_client
			.post(LoginForm::LOGIN_URL)
			.form(&self)
			.send()
			.await
			.and_then(reqwest::Response::error_for_status)?
			.text()
			.await?;

		if response
			.lines()
			.rfind(|&line| line == "is_valid:true")
			.is_none()
		{
			tracing::debug!(%response, "steam login invalid");
			return Err(Error::VerifyOpenIDPayload);
		}

		tracing::debug!("user logged in");

		Ok(self)
	}

	/// Extracts the SteamID from this form.
	pub fn steam_id(&self) -> SteamID
	{
		self.claimed_id
			.path_segments()
			.and_then(|segments| segments.last())
			.and_then(|segment| segment.parse::<SteamID>().ok())
			.expect("invalid response from steam")
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for OpenIDPayload
where
	S: Send + Sync + 'static,
	reqwest::Client: FromRef<S>,
{
	type Rejection = Error;

	async fn from_request_parts(
		req: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		let http_client = reqwest::Client::from_ref(state);

		Query::<Self>::from_request_parts(req, state)
			.map_err(Error::from)
			.and_then(|Query(payload)| payload.verify(&http_client))
			.await
	}
}
