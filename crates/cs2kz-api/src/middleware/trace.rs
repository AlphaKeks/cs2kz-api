use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ConnectInfo;
use axum::response::IntoResponse;
use axum::routing::Route;
use tower_http::classify::ServerErrorsFailureClass;
use tower_http::request_id::RequestId;
use tower_http::trace::TraceLayer;

pub fn layer() -> impl tower::Layer<
	Route,
	Service: tower::Service<
		crate::http::Request,
		Response: IntoResponse + 'static,
		Error: Into<Infallible> + 'static,
		Future: Send + 'static,
	> + Clone
	             + Send
	             + 'static,
> + Clone
       + Send
       + 'static
{
	TraceLayer::new_for_http()
		.make_span_with(make_span)
		.on_response(on_response)
		.on_failure(on_failure)
}

fn make_span(request: &crate::http::Request) -> tracing::Span
{
	let request_id = request
		.extensions()
		.get::<RequestId>()
		.map(tracing::field::debug);

	let peer_addr = request
		.extensions()
		.get::<ConnectInfo<SocketAddr>>()
		.map(|ConnectInfo(peer_addr)| tracing::field::debug(peer_addr));

	fn opt_field(value: Option<&impl tracing::Value>) -> &dyn tracing::Value
	{
		value.map_or(&tracing::field::Empty as _, |v| v as _)
	}

	let span = info_span! {
		target: "cs2kz_api::http",
		"request",
		request.id = opt_field(request_id.as_ref()),
		request.peer_addr = opt_field(peer_addr.as_ref()),
		request.method = ?request.method(),
		request.uri = %request.uri(),
		response.status = tracing::field::Empty,
		latency = tracing::field::Empty,
	};

	if let Some(ConnectInfo(peer_addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
		span.record("request.peer_addr", format_args!("{peer_addr}"));
	}

	span
}

fn on_response(response: &crate::http::Response, latency: Duration, span: &tracing::Span)
{
	span.record("response.status", format_args!("{}", response.status()))
		.record("latency", format_args!("{latency:?}"));
}

fn on_failure(failure: ServerErrorsFailureClass, _latency: Duration, _span: &tracing::Span)
{
	match failure {
		ServerErrorsFailureClass::Error(error) => {
			error!(target: "cs2kz_api::runtime::errors", %error, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_server_error() => {
			error!(target: "cs2kz_api::runtime::errors", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) if status.is_client_error() => {
			debug!(target: "cs2kz_api::runtime::errors", %status, "error occurred during request");
		}
		ServerErrorsFailureClass::StatusCode(status) => {
			warn!(target: "cs2kz_api::runtime::errors", %status, "error occurred during request");
		}
	}
}
