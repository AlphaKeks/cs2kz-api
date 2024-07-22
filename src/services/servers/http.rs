//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::ServerService;
use crate::runtime::Result;

impl From<ServerService> for Router
{
	fn from(svc: ServerService) -> Self
	{
		Router::new().with_state(svc)
	}
}
