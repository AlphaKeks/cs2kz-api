//! A service to manage login sessions.

use std::sync::Arc;

use derive_more::{Constructor, Debug};
use url::Url;

use crate::auth::SessionStore;
use crate::services::SteamService;

pub mod http;

/// A service for managing login sessions.
#[derive(Debug, Clone, Constructor)]
pub struct AuthService
{
	public_url: Arc<Url>,
	cookie_domain: Arc<str>,
	session_store: SessionStore,
	http_client: reqwest::Client,
	steam_service: SteamService,
}
