//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::RecordService;
use crate::runtime::Result;

impl From<RecordService> for Router
{
	fn from(svc: RecordService) -> Self
	{
		Router::new().with_state(svc)
	}
}
