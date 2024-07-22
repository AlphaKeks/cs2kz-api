//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::PlayerService;
use crate::runtime::Result;

impl From<PlayerService> for Router
{
	fn from(svc: PlayerService) -> Self
	{
		Router::new().with_state(svc)
	}
}
