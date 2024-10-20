use std::sync::Arc;

use axum::extract::{FromRef, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;

use crate::config::CookieConfig;
use crate::database::{self, DatabaseError};
use crate::http::responses::ErrorResponse;
use crate::users::sessions::authorization::{self, AuthorizeSession};
use crate::users::sessions::{self, Session};

/// A middleware for session authentication & authorization.
///
/// This function is intended to be consumed by [`axum::middleware::from_fn_with_state()`].
/// It will extract a session and authorize it according to `A`'s [`AuthorizeSession`]
/// implementation. If everything goes well, it will insert a [`Session`] object into the
/// [request's extensions] so that other middleware can access it later. After the inner service
/// is done, the session will either be extended or invalidated, based on whether
/// [`Session::invalidate()`] has been called at any point.
///
/// While [`Session`] can act as a standalone extractor without this middleware, it won't be
/// extended automatically that way. It is therefore recommended that this middleware is used
/// anywhere a [`Session`] is later extracted. [`Session`] will check the [request's extensions]
/// for an instance of itself before performing any logic, so it will never run more than once for
/// the same request.
///
/// [request's extensions]: http::Request::extensions
#[instrument(
	level = "debug",
	skip(authorization, pool, cookie_config, request, next),
	fields(authorization = ?std::any::type_name::<A>()),
	err(level = "debug")
)]
pub async fn session_auth<A>(
	State(SessionAuthState {
		mut authorization,
		pool,
		cookie_config,
	}): State<SessionAuthState<A>>,
	session: Session,
	mut cookies: CookieJar,
	request: Request,
	next: Next,
) -> Result<(CookieJar, Response), ErrorResponse>
where
	A: AuthorizeSession,
{
	let (mut parts, body) = request.into_parts();

	if let Err(error) = authorization.authorize_session(&mut parts, &session).await {
		return Ok((cookies, error.into_response()));
	}

	cookies = cookies.add(session.as_cookie(&cookie_config).into_owned());
	parts.extensions.insert(session.clone());

	let response = next.run(Request::from_parts(parts, body)).await;
	let mut txn = pool.begin_transaction().await?;

	if session.is_valid() {
		sessions::database::extend(&mut txn, sessions::database::ExtendSession {
			session_id: session.id(),
			duration: cookie_config.max_age,
		})
		.await?;
	} else {
		sessions::database::invalidate(&mut txn, session.id()).await?;
	}

	txn.commit().await.map_err(DatabaseError::from)?;

	Ok((cookies, response))
}

/// State required by the [`session_auth`] middleware.
#[derive(Clone)]
pub struct SessionAuthState<A = authorization::Noop> {
	authorization: A,
	pool: database::ConnectionPool,
	cookie_config: Arc<CookieConfig>,
}

impl SessionAuthState {
	pub fn new(
		pool: database::ConnectionPool,
		cookie_config: impl Into<Arc<CookieConfig>>,
	) -> Self {
		Self {
			authorization: authorization::Noop,
			pool,
			cookie_config: cookie_config.into(),
		}
	}
}

impl<A> SessionAuthState<A> {
	/// Changes the authorization strategy but preserves all other fields.
	pub fn with_authz<A2>(self, authorization: A2) -> SessionAuthState<A2>
	where
		A2: AuthorizeSession,
	{
		SessionAuthState {
			authorization,
			pool: self.pool,
			cookie_config: self.cookie_config,
		}
	}
}

impl<A> FromRef<SessionAuthState<A>> for database::ConnectionPool {
	fn from_ref(input: &SessionAuthState<A>) -> Self {
		input.pool.clone()
	}
}
