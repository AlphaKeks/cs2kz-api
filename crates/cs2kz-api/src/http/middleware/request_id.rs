use tower_http::request_id::{
	MakeRequestId,
	PropagateRequestIdLayer,
	RequestId,
	SetRequestIdLayer,
};
use uuid::fmt::Hyphenated;
use uuid::Uuid;

/// Creates a [`tower::Layer`], which produces a middleware that will generate an ID for every
/// request, injecting it both into its [extensions], as well as its headers.
///
/// [extensions]: http::Request::extensions
pub fn set_layer() -> SetRequestIdLayer<impl MakeRequestId + Clone> {
	SetRequestIdLayer::x_request_id(MakeUuidRequestId)
}

/// Creates a [`tower::Layer`], which produces a middleware that will forward the `x-request-id`
/// header from the request to the response.
///
/// This should be used together with [`set_layer()`].
pub fn propagate_layer() -> PropagateRequestIdLayer {
	PropagateRequestIdLayer::x_request_id()
}

/// Generates UUIDv7 request IDs.
#[derive(Clone, Copy)]
struct MakeUuidRequestId;

impl MakeRequestId for MakeUuidRequestId {
	fn make_request_id<B>(&mut self, _: &http::Request<B>) -> Option<RequestId> {
		let uuid = Uuid::now_v7();
		let mut buf = [0; Hyphenated::LENGTH];
		let uuid = uuid.as_hyphenated().encode_lower(&mut buf);

		let header_value =
			http::HeaderValue::from_str(uuid).expect("uuid should be a valid http header value");

		Some(RequestId::new(header_value))
	}
}
