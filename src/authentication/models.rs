//! Types for auth requests.

use serde::Deserialize;
use url::Url;
use utoipa::IntoParams;

/// Query parameters for the login endpoint.
#[derive(Debug, Deserialize, IntoParams)]
pub struct LoginRequest
{
	/// URL to redirect the user back to after a successful login.
	pub redirect_to: Url,
}

/// Query parameters for the logout endpoint.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct LogoutRequest
{
	/// Whether to invalidate all (still valid) sessions of this user.
	#[serde(default)]
	pub invalidate_all_sessions: bool,
}
