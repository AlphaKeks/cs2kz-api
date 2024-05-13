//! Middleware for logging incoming requests and outgoing responses.

use std::time::Duration;

use axum::extract::Request;
use axum::response::Response;
use tower_http::classify::ServerErrorsFailureClass;
use tracing::{debug, error, trace_span, warn};
use uuid::Uuid;

/// Creates a [`TraceLayer`] for logging HTTP requests.
///
/// [`TraceLayer`]: tower_http::trace::TraceLayer
macro_rules! layer {
	() => {
		::tower_http::trace::TraceLayer::new_for_http()
			.make_span_with($crate::http::middleware::logging::make_span_with)
			.on_response($crate::http::middleware::logging::on_response)
			.on_failure($crate::http::middleware::logging::on_failure)
	};
}

pub(crate) use layer;

#[doc(hidden)]
pub(crate) fn make_span_with(request: &Request) -> tracing::Span {
	trace_span! {
		target: "cs2kz_api::logging",
		"request",
		request.id = %Uuid::now_v7(),
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
pub(crate) fn on_response(response: &Response, latency: Duration, span: &tracing::Span) {
	span.record("response.status", format_args!("{}", response.status()))
		.record("response.headers", format_args!("{:?}", response.headers()))
		.record("latency", format_args!("{latency:?}"));
}

#[doc(hidden)]
pub(crate) fn on_failure(
	failure: ServerErrorsFailureClass,
	_latency: Duration,
	_span: &tracing::Span,
) {
	match failure {
		ServerErrorsFailureClass::StatusCode(status) if status.is_server_error() => {
			error!(target: "audit_log", %status, "request handler failed");
		}
		ServerErrorsFailureClass::StatusCode(status) => {
			debug!(%status, "request failed");
		}
		ServerErrorsFailureClass::Error(error) => {
			warn!(target: "audit_log", %error, "request handler failed");
		}
	}
}
