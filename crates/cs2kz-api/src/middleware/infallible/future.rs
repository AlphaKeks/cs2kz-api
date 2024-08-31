use std::convert;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use axum::response::IntoResponse;
use problem_details::AsProblemDetails;

/// Future for `<Infallible<S> as tower::Service>::Future`.
#[pin_project]
pub struct ResponseFuture<F, O, E>(#[pin] pub(super) F)
where
	F: Future<Output = Result<O, E>>;

impl<F, O, E> Future for ResponseFuture<F, O, E>
where
	F: Future<Output = Result<O, E>>,
	O: IntoResponse,
	E: AsProblemDetails<ProblemType = crate::http::Problem>,
{
	type Output = Result<crate::http::Response, convert::Infallible>;

	fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output>
	{
		self.project().0.poll(cx).map(|response| match response {
			Ok(v) => Ok(v.into_response()),
			Err(e) => Ok(e.as_problem_details().into_response()),
		})
	}
}
