use tower_http::request_id::{MakeRequestId, RequestId, SetRequestIdLayer};
use uuid::Uuid;

pub fn layer() -> SetRequestIdLayer<Uuidv7>
{
	SetRequestIdLayer::x_request_id(Uuidv7)
}

#[derive(Debug, Clone, Copy)]
pub struct Uuidv7;

impl MakeRequestId for Uuidv7
{
	fn make_request_id<B>(&mut self, _: &http::Request<B>) -> Option<RequestId>
	{
		Uuid::now_v7()
			.hyphenated()
			.to_string()
			.parse::<http::HeaderValue>()
			.map(RequestId::new)
			.ok()
	}
}
