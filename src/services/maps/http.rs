//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::MapService;
use crate::runtime::Result;

impl From<MapService> for Router
{
	fn from(svc: MapService) -> Self
	{
		Router::new().with_state(svc)
	}
}
