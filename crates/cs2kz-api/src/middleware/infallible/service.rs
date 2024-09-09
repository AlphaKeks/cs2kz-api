use std::task::Poll;
use std::{convert, task};

use derive_more::{Constructor, Debug};
use problem_details::AsProblemDetails;

use super::ResponseFuture;

/// A middleware that converts another service's `Error` to a [`Response`].
#[derive(Debug, Constructor, Clone)]
pub struct Infallible<S>
{
	/// The inner service.
	inner: S,
}

impl<S> tower::Service<crate::http::Request> for Infallible<S>
where
	S: tower::Service<crate::http::Request, Response = crate::http::Response>,
	S::Error: AsProblemDetails<ProblemType = crate::http::Problem>,
{
	type Response = crate::http::Response;
	type Error = convert::Infallible;
	type Future = ResponseFuture<S::Future, S::Response, S::Error>;

	fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		assert!(
			std::task::ready!(self.inner.poll_ready(cx)).is_ok(),
			"axum handlers are always ready"
		);
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, request: crate::http::Request) -> Self::Future
	{
		ResponseFuture(self.inner.call(request))
	}
}
