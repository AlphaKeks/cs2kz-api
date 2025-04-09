use {
	cs2kz_api::error::ResultExt,
	tower_http::request_id::{
		MakeRequestId,
		PropagateRequestIdLayer,
		RequestId,
		SetRequestIdLayer,
	},
	uuid::Uuid,
};

pub(crate) fn layers() -> (SetRequestIdLayer<impl MakeRequestId + Clone>, PropagateRequestIdLayer)
{
	(SetRequestIdLayer::x_request_id(MakeUuidv7RequestId), PropagateRequestIdLayer::x_request_id())
}

#[derive(Debug, Clone)]
struct MakeUuidv7RequestId;

impl MakeRequestId for MakeUuidv7RequestId
{
	fn make_request_id<B>(&mut self, _: &http::Request<B>) -> Option<RequestId>
	{
		Uuid::now_v7()
			.hyphenated()
			.to_string()
			.parse::<http::HeaderValue>()
			.inspect_err_dyn(|error| warn!(error))
			.map(RequestId::new)
			.ok()
	}
}
