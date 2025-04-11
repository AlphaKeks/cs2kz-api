use {
	cs2kz_api::error::ResultExt,
	tower_http::request_id::{
		MakeRequestId,
		PropagateRequestIdLayer,
		RequestId,
		SetRequestIdLayer,
	},
	ulid::Ulid,
};

pub(crate) fn layers() -> (SetRequestIdLayer<impl MakeRequestId + Clone>, PropagateRequestIdLayer)
{
	(SetRequestIdLayer::x_request_id(MakeUlidRequsetId), PropagateRequestIdLayer::x_request_id())
}

#[derive(Debug, Clone)]
struct MakeUlidRequsetId;

impl MakeRequestId for MakeUlidRequsetId
{
	fn make_request_id<B>(&mut self, _: &http::Request<B>) -> Option<RequestId>
	{
		Ulid::new()
			.to_string()
			.parse::<http::HeaderValue>()
			.inspect_err_dyn(|error| warn!(error))
			.map(RequestId::new)
			.ok()
	}
}
