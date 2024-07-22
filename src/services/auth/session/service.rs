//! A [`tower::Service`] for authenticating requests using sessions.
//!
//! See [module-level documentation] for more details.
//!
//! [module-level documentation]: crate::services::auth::session

use std::task::{self, Poll};

use axum::extract::Request;
use axum::http::{self, header};
use axum::response::{IntoResponse, Response};
use axum::RequestExt;
use futures::future::BoxFuture;

use super::{authorization, AuthorizeSession, Session, SessionRejection};
use crate::services::AuthService;

/// A layer producing the [`SessionManager`] middleware.
#[derive(Clone)]
pub struct SessionManagerLayer<A = authorization::None>
{
	/// For database and API config access.
	auth_svc: AuthService,

	/// The authorization strategy.
	authorization: A,
}

impl SessionManagerLayer
{
	/// Creates a new [`SessionManagerLayer`].
	pub fn new(auth_svc: AuthService) -> Self
	{
		Self { auth_svc, authorization: authorization::None }
	}

	/// Creates a new [`SessionManagerLayer`] with an authorization strategy.
	pub fn with_authorization<A>(auth_svc: AuthService, authorization: A)
	-> SessionManagerLayer<A>
	{
		SessionManagerLayer { auth_svc, authorization }
	}
}

impl<S, A> tower::Layer<S> for SessionManagerLayer<A>
where
	A: AuthorizeSession,
{
	type Service = SessionManager<S, A>;

	fn layer(&self, inner: S) -> Self::Service
	{
		SessionManager {
			auth_svc: self.auth_svc.clone(),
			authorization: self.authorization.clone(),
			inner,
		}
	}
}

/// Middleware for extracting and extending sessions.
#[derive(Clone)]
pub struct SessionManager<S, A = authorization::None>
{
	/// For database and API config access.
	auth_svc: AuthService,

	/// The authorization strategy.
	authorization: A,

	/// The inner service.
	inner: S,
}

/// Errors that can occur in the [`SessionManager`] middleware.
pub enum SessionManagerError<S, A = authorization::None>
where
	S: tower::Service<Request, Response = Response, Error: IntoResponse>,
	A: AuthorizeSession,
{
	/// Extracting the session failed.
	Session(SessionRejection),

	/// Authorization failed.
	Authorize(<A as AuthorizeSession>::Error),

	/// The underlying service failed.
	Service(<S as tower::Service<Request>>::Error),
}

impl<S, A> IntoResponse for SessionManagerError<S, A>
where
	S: tower::Service<Request, Response = Response, Error: IntoResponse>,
	A: AuthorizeSession,
{
	fn into_response(self) -> Response
	{
		match self {
			Self::Session(rej) => rej.into_response(),
			Self::Authorize(err) => err.into_response(),
			Self::Service(err) => err.into_response(),
		}
	}
}

impl<S, A> tower::Service<Request> for SessionManager<S, A>
where
	S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
	<S as tower::Service<Request>>::Future: Send,
	<S as tower::Service<Request>>::Error: IntoResponse,
	A: AuthorizeSession,
{
	type Response = Response;
	type Error = SessionManagerError<S, A>;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, cx: &mut task::Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner
			.poll_ready(cx)
			.map_err(SessionManagerError::Service)
	}

	fn call(&mut self, mut req: Request) -> Self::Future
	{
		let auth_svc = self.auth_svc.clone();
		let authorization = self.authorization.clone();
		let mut inner = self.inner.clone();

		Box::pin(async move {
			let session: Session = req
				.extract_parts_with_state(&auth_svc.database)
				.await
				.map_err(SessionManagerError::Session)?;

			req.extensions_mut().insert(session.clone());

			authorization
				.authorize_session(&session, &mut req)
				.await
				.map_err(SessionManagerError::Authorize)?;

			let mut response = inner
				.call(req)
				.await
				.map_err(SessionManagerError::Service)?;

			let session_cookie = session
				.into_cookie(&auth_svc.api_config)
				.encoded()
				.to_string()
				.parse::<http::HeaderValue>()
				.expect("valid cookie");

			response
				.headers_mut()
				.insert(header::SET_COOKIE, session_cookie);

			Ok(response)
		})
	}
}
