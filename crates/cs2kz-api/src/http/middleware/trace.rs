use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ConnectInfo;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::RequestId;
use tower_http::trace::{
	DefaultOnBodyChunk,
	DefaultOnEos,
	HttpMakeClassifier,
	MakeSpan,
	OnFailure,
	OnRequest,
	OnResponse,
	TraceLayer,
};
use tracing::field;

/// Creates a [`tower::Layer`], which produces a middleware that will emit tracing spans & events
/// for every HTTP request.
///
/// The `include_headers` parameter can be used to control whether request/response headers should
/// be included in the traces.
pub fn layer<B>(
	include_headers: bool,
) -> TraceLayer<
	HttpMakeClassifier,
	impl MakeSpan<B> + Clone,
	impl OnRequest<B> + Clone,
	impl OnResponse<B> + Clone,
	DefaultOnBodyChunk,
	DefaultOnEos,
	impl OnFailure<ServerErrorsFailureClass> + Clone,
> {
	let on_request = move |req: &http::Request<B>, span: &tracing::Span| {
		on_request(req, span, include_headers);
	};

	let on_response = move |res: &http::Response<B>, latency: Duration, span: &tracing::Span| {
		on_response(res, latency, span, include_headers);
	};

	TraceLayer::new_for_http()
		.make_span_with(make_span)
		.on_request(on_request)
		.on_response(on_response)
		.on_failure(on_failure)
}

/// Creates each request span.
///
/// # Panics
///
/// This function assumes two things:
///    1. Another middleware runs before it, namely `RequestIdLayer`. It generates a unique ID that
///       is inserted into both the request's headers and extensions. We want to include this ID in
///       traces even if headers are not recorded, so `make_span` will extract it.
///    2. The `TraceLayer` using this function is part of a service stack that is eventually passed
///       to an `axum::Router` that is turned into a `tower::Service` via
///       `axum::Router::into_make_service_with_connect_info::<SocketAddr>()`.
///       This will ensure a `ConnectInfo` value is inserted into the request extensions.
///       This value contains the client address and port of the TCP connection.
///
/// If any of the above assumptions are not true, this function will panic.
fn make_span<B>(request: &http::Request<B>) -> tracing::Span {
	let request_id = request
		.extensions()
		.get::<RequestId>()
		.expect("`RequestId` should have been injected by a `RequestIdLayer`")
		.header_value()
		.to_str()
		.expect("request id should be a UUID and therefore valid UTF-8");

	let client_addr = request
		.extensions()
		.get::<ConnectInfo<SocketAddr>>()
		.map(|&ConnectInfo(addr)| addr)
		.expect("`ConnectInfo<SocketAddr>` should have been injected by the router");

	info_span! {
		target: "cs2kz_api::http",
		"request",
		request.id = %request_id,
		request.client_addr = %client_addr,
		request.method = field::Empty,
		request.uri = field::Empty,
		request.headers = field::Empty,
		response.status = field::Empty,
		response.headers = field::Empty,
		latency = field::Empty,
	}
}

/// Records metadata about the request.
fn on_request<B>(request: &http::Request<B>, span: &tracing::Span, include_headers: bool) {
	span.record("request.method", field::display(request.method()));
	span.record("request.uri", field::display(request.uri()));

	if include_headers {
		span.record("request.headers", field::debug(request.headers()));
	}
}

/// Records metadata about the response.
fn on_response<B>(
	response: &http::Response<B>,
	latency: Duration,
	span: &tracing::Span,
	include_headers: bool,
) {
	span.record("response.status", field::display(response.status()));

	if include_headers {
		span.record("response.headers", field::debug(response.headers()));
	}

	span.record("latency", field::debug(latency));
}

/// Called whenever a request "failed".
///
/// What qualifies as a "failure" is determined by `ServerErrorsFailureClass`, but generally it is
/// a `500 Internal Server Error` status code.
fn on_failure(failure_class: ServerErrorsFailureClass, latency: Duration, span: &tracing::Span) {
	span.in_scope(|| match failure_class {
		ServerErrorsFailureClass::StatusCode(status) => {
			error!(%status, ?latency, "http handler failed");
		}
		ServerErrorsFailureClass::Error(error) => {
			error!(%error, ?latency, "http handler failed");
		}
	})
}
