use std::any::Any;
use std::borrow::Cow;

use axum::response::IntoResponse;
use problem_details::AsProblemDetails;
use thiserror::Error;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

pub fn layer() -> CatchPanicLayer<PanicHandler>
{
	CatchPanicLayer::custom(PanicHandler)
}

#[derive(Debug, Clone, Copy)]
pub struct PanicHandler;

#[derive(Debug, PartialEq, Error)]
#[error("something went wrong; please report this incident")]
struct PanicRejection;

impl AsProblemDetails for PanicRejection
{
	type ProblemType = crate::http::Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		crate::http::Problem::Internal
	}

	fn detail(&self) -> Cow<'static, str>
	{
		Cow::Borrowed("something went wrong; please report this incident")
	}
}

impl ResponseForPanic for PanicHandler
{
	type ResponseBody = crate::http::Body;

	fn response_for_panic(
		&mut self,
		error: Box<dyn Any + Send + 'static>,
	) -> http::Response<Self::ResponseBody>
	{
		let error = error
			.downcast_ref::<&str>()
			.copied()
			.or_else(|| error.downcast_ref::<String>().map(String::as_str));

		error!(?error, "http handler panicked");

		PanicRejection.as_problem_details().into_response()
	}
}
