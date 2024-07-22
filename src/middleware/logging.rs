//! A middleware for logging incoming requests.

use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::{ConnectInfo, Request};
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use uuid::Uuid;

/// Creates a tower service that will log incoming HTTP requests.
macro_rules! layer {
	() => {
		tower_http::trace::TraceLayer::new_for_http()
			.make_span_with($crate::middleware::logging::make_span)
			.on_response($crate::middleware::logging::on_response)
			.on_failure($crate::middleware::logging::on_failure)
	};
}

pub(crate) use layer;

#[doc(hidden)]
pub(crate) fn make_span(request: &Request) -> tracing::Span
{
	let ip = match request.extensions().get::<ConnectInfo<SocketAddr>>() {
		None => String::from("N/A"),
		Some(ConnectInfo(addr)) => addr.to_string(),
	};

	tracing::info_span! {
		target: "cs2kz_api::http",
		"request",
		request.id = %Uuid::now_v7(),
		request.ip = %ip,
		request.method = %request.method(),
		request.uri = %request.uri(),
		request.version = ?request.version(),
		request.headers = ?request.headers(),
		response.status = tracing::field::Empty,
		response.headers = tracing::field::Empty,
		latency = tracing::field::Empty,
	}
}

#[doc(hidden)]
pub(crate) fn on_response(response: &Response, latency: Duration, span: &tracing::Span)
{
	span.record("response.status", format_args!("{}", response.status()))
		.record("response.headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{:?}", latency));
}

#[doc(hidden)]
pub(crate) fn on_failure(
	failure: ServerErrorsFailureClass,
	_latency: Duration,
	_span: &tracing::Span,
)
{
	match failure {
		ServerErrorsFailureClass::Error(error) => {
			tracing::error!(target: "cs2kz_api::audit_log", %error, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_server_error() => {
			tracing::error!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_client_error() => {
			tracing::debug!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) => {
			tracing::warn!(target: "cs2kz_api::audit_log", %status, "error occurred during request");
		}
	}
}
