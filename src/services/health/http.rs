//! HTTP handlers for this service.

use axum::extract::State;
use axum::{routing, Router};

use super::HealthService;

impl From<HealthService> for Router
{
	fn from(svc: HealthService) -> Self
	{
		Router::new().route("/", routing::get(get)).with_state(svc)
	}
}

/// (͡ ͡° ͜ つ ͡͡°)
async fn get(State(svc): State<HealthService>) -> &'static str
{
	svc.hello().await
}
