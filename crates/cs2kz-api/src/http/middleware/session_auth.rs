//! Session authentication & authorization.

use std::fmt;
use std::sync::Arc;

use axum::extract::{FromRef, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum_extra::extract::CookieJar;

use crate::config::CookieConfig;
use crate::database;
use crate::http::response::ErrorResponse;
use crate::users::sessions::authorization::{self, AuthorizeSession};
use crate::users::sessions::{self, Session};

/// State required by the session authentication middleware.
#[derive(Debug, Clone)]
pub struct SessionAuthState<A = authorization::None> {
	database: database::ConnectionPool,
	cookie_config: Arc<CookieConfig>,
	pub(in crate::http) authorization: A,
}

/// Performs session authentication.
///
/// Requests that fail to authenticate or authorize will be rejected
/// immediately. Authorization is controlled via the `A` generic type parameter
/// on [`SessionAuthState`]. Sessions will be extended / expired depending on
/// whether the inner service calls [`Session::invalidate()`] during its
/// execution.
#[instrument(level = "debug", skip(request, next))]
pub async fn session_auth<A>(
	State(mut state): State<SessionAuthState<A>>,
	session: Session,
	cookies: CookieJar,
	request: Request,
	next: Next,
) -> Result<Response, ErrorResponse>
where
	A: AuthorizeSession + fmt::Debug,
{
	let (mut parts, body) = request.into_parts();

	if let Err(error) = state
		.authorization
		.authorize_session(&session, &mut parts)
		.await
	{
		return Ok(error.into_response());
	}

	let response = next.run(Request::from_parts(parts, body)).await;
	let cookie = session.as_cookie(&state.cookie_config);

	if !session.is_valid() {
		let mut conn = state.database.get_connection().await?;
		sessions::invalidate(&mut conn, session.id()).await?;
	}

	Ok((cookies.add(cookie), response).into_response())
}

impl SessionAuthState {
	/// Creates a new [`SessionAuthState`] _without_ authorization.
	///
	/// An authorization strategy can be provided via
	/// [`SessionAuthState::with_authorization()`].
	pub fn new(
		database: database::ConnectionPool,
		cookie_config: impl Into<Arc<CookieConfig>>,
	) -> Self {
		Self {
			database,
			cookie_config: cookie_config.into(),
			authorization: authorization::None,
		}
	}
}

impl<A> SessionAuthState<A> {
	/// Changes the authorization strategy.
	pub fn with_authorization<A2>(self, authorization: A2) -> SessionAuthState<A2>
	where
		A2: AuthorizeSession,
	{
		SessionAuthState {
			database: self.database,
			cookie_config: self.cookie_config,
			authorization,
		}
	}
}

impl<A> FromRef<SessionAuthState<A>> for database::ConnectionPool {
	fn from_ref(input: &SessionAuthState<A>) -> Self {
		input.database.clone()
	}
}
