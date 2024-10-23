//! This module contains the [`LoginForm`] and related types.

use serde::Serialize;
use url::Url;

/// OpenID 2.0 parameters that will be sent as a URL query when redirecting to
/// Steam.
///
/// See [crate-level documentation] for more details.
///
/// [crate-level documentation]: crate
#[derive(Debug, Clone, Serialize)]
#[expect(missing_docs, reason = "should be self-explanatory")]
pub struct LoginForm {
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

impl LoginForm {
	/// Creates a new [`LoginForm`].
	///
	/// The `realm` parameter should be the base URL Steam should send its
	/// response to. `callback_route` will be appended to the base URL, and the
	/// result is the URL Steam will send its response to.
	///
	/// # Panics
	///
	/// This function will panic if `realm` and `callback_route` can't be joined
	/// into a URL.
	pub fn new(realm: Url, callback_route: &str) -> Self {
		let return_to = realm
			.join(callback_route)
			.expect("`realm` + `callback_route` should produce a valid URL");

		Self {
			namespace: "https://specs.openid.net/auth/2.0",
			identity: "https://specs.openid.net/auth/2.0/identifier_select",
			claimed_id: "https://specs.openid.net/auth/2.0/identifier_select",
			mode: "checkid_setup",
			realm,
			return_to,
		}
	}

	/// Constructs a URL you can redirect your users to for the login process.
	///
	/// # Errors
	///
	/// This function will return an error if `userdata` cannot be serialized as
	/// a URL query parameter.
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
		let mut url = Url::parse(crate::LOGIN_URL).expect("valid url");
		url.set_query(Some(&query_string));

		Ok(url)
	}
}

/// Error returned by [`LoginForm::redirect_url()`].
#[derive(Debug, Error)]
#[error("failed to encode userdata as url query parameter: {0}")]
pub struct CreateRedirectUrlError(#[from] serde_urlencoded::ser::Error);
