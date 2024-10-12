use std::any::type_name;

use axum::extract::{FromRef, FromRequestParts, Request, State};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::RequestExt;

use super::authorization::AuthorizeSession;
use super::{Session, SessionRejection};
use crate::database;

/// A middleware that will authenticate and authorize the user.
///
/// If `REQUIRE_AUTHORIZATION` is explicitly set to `false`, an authorization failure will **not**
/// reject the request, and the inner service will still be called.
/// However, [`Session::is_authorized()`] will then return `false`.
///
/// A [`Session`] object is inserted into the [request extensions], so that the inner service can
/// access the user's information.
///
/// The user's session will be extended after the inner service is done.
/// If [`Session::invalidate()`] has been called, the session will be invalidated instead.
///
/// [request extensions]: http::Request::extensions
#[instrument(skip(req, next), err(Debug, level = "debug"))]
pub async fn session_auth<A, const REQUIRE_AUTHORIZATION: bool>(
	State(mut state): State<SessionAuthState<A, REQUIRE_AUTHORIZATION>>,
	req: Request,
	next: Next,
) -> Result<Response, SessionRejection>
where
	A: AuthorizeSession,
{
	let (mut parts, body) = req.into_parts();
	let session = Session::from_request_parts(&mut parts, &state).await?;
	let authorization = state
		.authorization
		.authorize_session(&mut parts, &session)
		.await;

	match (authorization, REQUIRE_AUTHORIZATION) {
		(Ok(()), _) => session.authorize(),
		(Err(_), false) => {}
		(Err(rejection), true) => return Ok(rejection.into_response()),
	}

	Ok(next.run(Request::from_parts(parts, body)).await)
}

#[derive(Debug, Clone)]
pub struct SessionAuthState<A, const REQUIRE_AUTHORIZATION: bool = true> {
	#[debug("`{}`{}", type_name::<A>(), if REQUIRE_AUTHORIZATION { " (required)" } else { "" })]
	authorization: A,

	#[debug("MySQL")]
	database: database::ConnectionPool,
}

impl<A> SessionAuthState<A>
where
	A: AuthorizeSession,
{
	pub fn new(authorization: A, database: database::ConnectionPool) -> Self {
		Self {
			authorization,
			database,
		}
	}
}

impl<A, const REQUIRE_AUTHORIZATION: bool> FromRef<SessionAuthState<A, REQUIRE_AUTHORIZATION>>
	for database::ConnectionPool
{
	fn from_ref(input: &SessionAuthState<A, REQUIRE_AUTHORIZATION>) -> Self {
		input.database.clone()
	}
}
