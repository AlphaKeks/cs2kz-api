//! Request / Response types for this service.

use serde::Deserialize;
use url::Url;

use super::Session;

/// Request payload for logging in with Steam.
#[derive(Debug, Deserialize)]
pub struct LoginRequest
{
	/// URL to redirect to after the login process is complete.
	pub redirect_to: Url,
}

/// Response payload for logging in with Steam.
#[derive(Debug, Deserialize)]
pub struct LoginResponse
{
	/// OpenID URL to redirect the user to so they can login.
	pub openid_url: Url,
}

/// Request payload for logging in with Steam.
#[derive(Debug)]
pub struct LogoutRequest
{
	/// Whether to invalidate all previous sessions, rather than just the
	/// current one.
	pub invalidate_all_sessions: bool,

	/// The user's session.
	pub session: Session,
}
