use {
	axum::body::HttpBody,
	bytes::Buf,
	http::{Request, Response},
	std::time::Duration,
	tower_http::{
		classify::ServerErrorsFailureClass,
		request_id::RequestId,
		trace::{
			HttpMakeClassifier,
			MakeSpan,
			OnBodyChunk,
			OnEos,
			OnFailure,
			OnRequest,
			OnResponse,
			TraceLayer,
		},
	},
};

pub(crate) fn layer<ReqBody, ResBody>(
	include_headers: bool,
) -> TraceLayer<
	HttpMakeClassifier,
	impl MakeSpan<ReqBody> + Clone,
	impl OnRequest<ReqBody> + Clone,
	impl OnResponse<ResBody> + Clone,
	impl OnBodyChunk<ResBody::Data> + Clone,
	impl OnEos + Clone,
	impl OnFailure<ServerErrorsFailureClass> + Clone,
>
where
	ResBody: HttpBody,
{
	TraceLayer::new_for_http()
		.make_span_with(make_span::<ReqBody>)
		.on_request(move |req: &Request<ReqBody>, span: &tracing::Span| {
			on_request(req, span, include_headers)
		})
		.on_response(move |res: &Response<ResBody>, latency: Duration, span: &tracing::Span| {
			on_response(res, latency, span, include_headers)
		})
		.on_body_chunk(on_body_chunk::<ResBody>)
		.on_eos(on_eos)
		.on_failure(on_failure)
}

fn make_span<B>(_: &Request<B>) -> tracing::Span
{
	tracing::info_span!(
		target: "cs2kz_api::http",
		"request",
		req.id = tracing::field::Empty,
		req.method = tracing::field::Empty,
		req.uri = tracing::field::Empty,
		req.version = tracing::field::Empty,
		req.headers = tracing::field::Empty,
		res.status = tracing::field::Empty,
		res.headers = tracing::field::Empty,
	)
}

fn on_request<B>(req: &Request<B>, span: &tracing::Span, include_headers: bool)
{
	if let Some(request_id) = req.extensions().get::<RequestId>() {
		span.record("req.id", tracing::field::debug(request_id.header_value()));
	} else {
		warn!(target: "cs2kz_api::http::request", "no request ID in request extensions");
	}

	span.record("req.method", tracing::field::debug(req.method()));
	span.record("req.uri", tracing::field::display(req.uri()));
	span.record("req.version", tracing::field::debug(req.version()));

	if include_headers {
		span.record("req.headers", tracing::field::debug(req.headers()));
	}

	info!(target: "cs2kz_api::http", "starting to process request");
}

fn on_response<B>(res: &Response<B>, latency: Duration, span: &tracing::Span, include_headers: bool)
{
	span.record("res.status", res.status().as_u16());

	if include_headers {
		span.record("res.headers", tracing::field::debug(res.headers()));
	}

	info!(target: "cs2kz_api::http", ?latency, "finished processing request");
}

fn on_body_chunk<B: HttpBody>(chunk: &B::Data, latency: Duration, _span: &tracing::Span)
{
	trace!(
		target: "cs2kz_api::http::response::body",
		size = chunk.remaining(),
		?latency,
		"sending body chunk",
	);
}

fn on_eos(trailers: Option<&http::HeaderMap>, stream_duration: Duration, _span: &tracing::Span)
{
	trace!(
		target: "cs2kz_api::http::response::body",
		?trailers,
		?stream_duration,
		"reached end of body stream",
	);
}

fn on_failure(failure_class: ServerErrorsFailureClass, latency: Duration, _span: &tracing::Span)
{
	match failure_class {
		ServerErrorsFailureClass::StatusCode(status) => {
			error!(
				target: "cs2kz_api::http::error",
				status = status.as_u16(),
				?latency,
				"failed to handle request",
			);
		},
		ServerErrorsFailureClass::Error(error) => {
			error!(
				target: "cs2kz_api::http::error",
				error,
				?latency,
				"failed to handle request",
			);
		},
	}
}
