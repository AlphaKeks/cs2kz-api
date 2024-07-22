//! A middleware that maps any fallible [`tower::Service`] to a service with
//! `Error = Infallible` (assuming `Error: IntoResponse`).

use std::convert;
use std::future::Future;
use std::pin::Pin;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};

/// A layer producing the [`Infallible`] service.
#[derive(Clone)]
pub struct InfallibleLayer;

impl<S> tower::Layer<S> for InfallibleLayer
{
	type Service = Infallible<S>;

	fn layer(&self, inner: S) -> Self::Service
	{
		Infallible { inner }
	}
}

/// A middleware that converts another service's `Error` to a [`Response`].
#[derive(Clone)]
pub struct Infallible<S>
{
	/// The inner service.
	inner: S,
}

impl<S> tower::Service<Request> for Infallible<S>
where
	S: tower::Service<Request, Response = Response>,
	S::Error: IntoResponse,
{
	type Response = Response;
	type Error = convert::Infallible;
	type Future = InfallibleFuture<S::Future, S::Response, S::Error>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		assert!(task::ready!(self.inner.poll_ready(cx)).is_ok(), "axum handlers are always ready");
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request) -> Self::Future
	{
		InfallibleFuture(self.inner.call(req))
	}
}

/// Future for `<Infallible<S> as tower::Service>::Future`.
#[pin_project]
pub struct InfallibleFuture<F, R, E>(#[pin] F)
where
	F: Future<Output = Result<R, E>>;

impl<F, R, E> Future for InfallibleFuture<F, R, E>
where
	F: Future<Output = Result<R, E>>,
	R: IntoResponse,
	E: IntoResponse,
{
	type Output = Result<Response, convert::Infallible>;

	fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output>
	{
		self.project().0.poll(cx).map(|res| match res {
			Ok(v) => Ok(v.into_response()),
			Err(e) => Ok(e.into_response()),
		})
	}
}
