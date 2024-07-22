//! A very basic service that acts as a healthcheck.
//!
//! This doesn't really need to be a service, but it's the simplest example of
//! one, and can be used as a reference for writing new services.

use axum::extract::FromRef;

mod http;

/// A service that simply responds if the API is healthy.
#[derive(Clone, Copy, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct HealthService {}

impl HealthService
{
	/// Create a new [`HealthService`].
	pub fn new() -> Self
	{
		Self {}
	}

	/// Says hello to the world.
	#[tracing::instrument(level = "trace", skip(self))]
	pub async fn hello(&self) -> &'static str
	{
		"(͡ ͡° ͜ つ ͡͡°)"
	}
}
