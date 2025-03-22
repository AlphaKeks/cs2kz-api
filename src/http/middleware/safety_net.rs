use std::{
	convert::Infallible,
	error::Error,
	future,
	io,
	mem,
	pin::Pin,
	task::{self, Poll, ready},
	time::Duration,
};

use axum::response::IntoResponse;
use tokio_util::sync::WaitForCancellationFutureOwned;

use crate::TaskManager;

pub(crate) fn layer(task_manager: TaskManager, timeout: Duration) -> SafetyNetLayer
{
	SafetyNetLayer { task_manager, timeout }
}

#[derive(Debug, Clone)]
pub(crate) struct SafetyNetLayer
{
	task_manager: TaskManager,
	timeout: Duration,
}

impl<S> tower::Layer<S> for SafetyNetLayer
{
	type Service = SafetyNet<S>;

	fn layer(&self, inner: S) -> Self::Service
	{
		SafetyNet {
			inner,
			task_manager: self.task_manager.clone(),
			timeout: self.timeout,
		}
	}
}

#[derive(Debug, Clone)]
pub(crate) struct SafetyNet<S>
{
	inner: S,
	task_manager: TaskManager,
	timeout: Duration,
}

#[pin_project]
pub(crate) struct ResponseFuture<T, E>
where
	T: IntoResponse + Send + 'static,
	E: IntoResponse + Send + 'static,
{
	#[pin]
	kind: ResponseFutureKind<T, E>,
}

#[pin_project(project = ResponseFutureProj)]
enum ResponseFutureKind<T, E>
where
	T: IntoResponse + Send + 'static,
	E: IntoResponse + Send + 'static,
{
	Error(io::Error),
	Spawned
	{
		span: tracing::Span,
		#[pin]
		cancellation: WaitForCancellationFutureOwned,
		task_handle: tokio::task::JoinHandle<Result<Result<T, E>, tokio::time::error::Elapsed>>,
	},
}

impl<S, B> tower::Service<http::Request<B>> for SafetyNet<S>
where
	S: tower::Service<http::Request<B>, Response = axum::response::Response>
		+ Clone
		+ Send
		+ 'static,
	S::Error: IntoResponse + Send + 'static,
	S::Future: Send + 'static,
	B: Send + 'static,
{
	type Response = S::Response;
	type Error = Infallible;
	type Future = ResponseFuture<S::Response, S::Error>;

	fn poll_ready(&mut self, _: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: http::Request<B>) -> Self::Future
	{
		let mut service = self.inner.clone();
		mem::swap(&mut service, &mut self.inner);

		let future = async move {
			if let Err(err) = future::poll_fn(|cx| service.poll_ready(cx)).await {
				return Ok(err.into_response());
			}

			service.call(req).await
		};

		ResponseFuture::new(&self.task_manager, self.timeout, future)
	}
}

impl<T, E> ResponseFuture<T, E>
where
	T: IntoResponse + Send + 'static,
	E: IntoResponse + Send + 'static,
{
	fn new<F>(task_manager: &TaskManager, timeout: Duration, future: F) -> Self
	where
		F: IntoFuture<Output = Result<T, E>>,
		F::IntoFuture: Send + 'static,
	{
		let future_span = tracing::debug_span!(parent: None, "safety_net");
		let task_span = tracing::trace_span!(parent: None, "handler");
		task_span.follows_from(&future_span);

		Self {
			kind: task_manager
				.spawn(task_span, |_| tokio::time::timeout(timeout, future))
				.map_or_else(ResponseFutureKind::Error, |task_handle| {
					ResponseFutureKind::Spawned {
						span: future_span,
						cancellation: task_manager.cancellation_token().cancelled_owned(),
						task_handle,
					}
				}),
		}
	}
}

impl<T, E> Future for ResponseFuture<T, E>
where
	T: IntoResponse + Send + 'static,
	E: IntoResponse + Send + 'static,
{
	type Output = Result<axum::response::Response, Infallible>;

	fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output>
	{
		Poll::Ready(Ok(match self.project().kind.project() {
			ResponseFutureProj::Error(error) => {
				tracing::warn!(error = &*error as &dyn Error, "failed to spawn handler task");
				http::StatusCode::SERVICE_UNAVAILABLE.into_response()
			},
			ResponseFutureProj::Spawned { span, cancellation, task_handle } => 'scope: {
				let _guard = span.enter();

				if cancellation.poll(cx).is_ready() {
					tracing::trace!("server shutting down");
					break 'scope http::StatusCode::SERVICE_UNAVAILABLE.into_response();
				}

				match ready!(Pin::new(task_handle).poll(cx)) {
					Ok(Ok(output)) => output.into_response(),
					Ok(Err(_)) => {
						tracing::warn!("service call timed out");
						http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
					},
					Err(_) => {
						tracing::error!("service call panicked");
						http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
					},
				}
			},
		}))
	}
}
