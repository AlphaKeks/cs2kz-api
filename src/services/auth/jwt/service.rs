//! A [`tower::Service`] contain an `Authorization` header with a valid JWT.

use std::marker::PhantomData;
use std::task::{self, Poll};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use futures::future::BoxFuture;
use serde::de::DeserializeOwned;
use tap::Pipe;

use super::{Jwt, JwtRejection};
use crate::services::AuthService;

/// A layer producing the [`JwtService`] middleware.
pub struct JwtLayer<T>
where
	T: DeserializeOwned,
{
	/// For decoding JWTs.
	auth_svc: AuthService,

	/// The payload type of the JWTs we're gonna be decoding.
	_marker: PhantomData<T>,
}

impl<T> JwtLayer<T>
where
	T: DeserializeOwned,
{
	/// Creates a new [`JwtLayer`].
	pub fn new(auth_svc: AuthService) -> Self
	{
		Self { auth_svc, _marker: PhantomData }
	}
}

impl<T> Clone for JwtLayer<T>
where
	T: DeserializeOwned,
{
	fn clone(&self) -> Self
	{
		Self::new(self.auth_svc.clone())
	}
}

impl<S, T> tower::Layer<S> for JwtLayer<T>
where
	T: DeserializeOwned,
{
	type Service = JwtService<S, T>;

	fn layer(&self, inner: S) -> Self::Service
	{
		JwtService { auth_svc: self.auth_svc.clone(), _marker: PhantomData, inner }
	}
}

pub struct JwtService<S, T>
where
	T: DeserializeOwned,
{
	/// For decoding JWTs.
	auth_svc: AuthService,

	/// The payload type of the JWTs we're gonna be decoding.
	_marker: PhantomData<T>,

	/// The inner service we're wrapping.
	inner: S,
}

impl<S, T> Clone for JwtService<S, T>
where
	S: Clone,
	T: DeserializeOwned,
{
	fn clone(&self) -> Self
	{
		Self { auth_svc: self.auth_svc.clone(), _marker: PhantomData, inner: self.inner.clone() }
	}
}

pub enum JwtServiceError<S>
where
	S: tower::Service<Request, Response = Response, Error: IntoResponse>,
{
	/// We failed to extract the JWT.
	Jwt(JwtRejection),

	/// The underlying service failed for some reason.
	Service(<S as tower::Service<Request>>::Error),
}

impl<S> IntoResponse for JwtServiceError<S>
where
	S: tower::Service<Request, Response = Response, Error: IntoResponse>,
{
	fn into_response(self) -> Response
	{
		match self {
			Self::Jwt(rej) => rej.into_response(),
			Self::Service(error) => error.into_response(),
		}
	}
}

impl<S, T> tower::Service<Request> for JwtService<S, T>
where
	S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
	<S as tower::Service<Request>>::Future: Send,
	<S as tower::Service<Request>>::Error: IntoResponse,
	T: Clone + DeserializeOwned + Send + Sync + 'static,
{
	type Response = Response;
	type Error = JwtServiceError<S>;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner.poll_ready(cx).map_err(JwtServiceError::Service)
	}

	fn call(&mut self, mut req: Request) -> Self::Future
	{
		let auth_svc = self.auth_svc.clone();
		let mut inner = self.inner.clone();

		Box::pin(async move {
			req.extract_parts_with_state::<Jwt<T>, _>(&auth_svc)
				.await
				.map_err(JwtServiceError::Jwt)?
				.pipe(|jwt| req.extensions_mut().insert(jwt));

			let response = inner.call(req).await.map_err(JwtServiceError::Service)?;

			Ok(response)
		})
	}
}
