//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::PluginService;
use crate::runtime::Result;

impl From<PluginService> for Router
{
	fn from(svc: PluginService) -> Self
	{
		Router::new().with_state(svc)
	}
}
