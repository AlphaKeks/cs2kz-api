//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::SteamService;
use crate::runtime::Result;

impl From<SteamService> for Router
{
	fn from(svc: SteamService) -> Self
	{
		Router::new().with_state(svc)
	}
}
