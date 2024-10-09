//! Everything related to user sessions.
//!
//! This is the primary authentication mechanism provided by the API for individual users.

use std::fmt;
use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum_extra::extract::cookie::Cookie;

use self::state::State;
use crate::users::permissions::Permissions;
use crate::users::UserID;

mod id;
pub use id::SessionID;

mod state;
mod database;

mod rejection;
pub use rejection::SessionRejection;

pub mod authorization;
pub mod middleware;

/// A user session.
///
/// This type acts as an [extractor].
///
/// [extractor]: axum::extract
#[derive(Debug, Clone)]
pub struct Session {
	id: SessionID,
	user_id: UserID,
	user_permissions: Permissions,
	state: Arc<State>,
}

impl Session {
	/// Invalidates this session.
	///
	/// This will cause the user's session cookie to be removed, and the database entry for
	/// this session to be expired. This does not happen immediately, but in the authentication
	/// middleware, after the request handler has already returned.
	pub fn invalidate(&self) {
		self.state.set(State::invalidated());
	}

	fn authorize(&self) {
		self.state.set(State::authorized());
	}

	fn new(user_id: UserID, user_permissions: Permissions) -> Self {
		Self {
			id: SessionID::new(),
			user_id,
			user_permissions,
			state: Arc::new(State::authenticated()),
		}
	}
}

impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync,
	crate::database::ConnectionPool: FromRef<S>,
{
	type Rejection = SessionRejection;

	async fn from_request_parts(
		req: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		if let Some(cached) = req.extensions.get::<Self>().cloned() {
			return Ok(cached);
		}

		let mut txn = crate::database::ConnectionPool::from_ref(state)
			.begin()
			.await?;

		let session_id = extract_session_id(&req.headers)?;
		let session = database::get_by_id(&mut txn, session_id)
			.await?
			.ok_or(SessionRejection::InvalidSessionID)?;

		let session = Session::new(session.user_id, session.user_permissions);

		req.extensions.insert(session.clone());

		Ok(session)
	}
}

#[instrument(
	skip(headers),
	fields(cookies = tracing::field::Empty),
	ret(level = "debug"),
	err(level = "debug"),
)]
fn extract_session_id(headers: &http::HeaderMap) -> Result<SessionID, SessionRejection> {
	const COOKIE_NAME: &str = "kz-auth";

	// TODO: replace `DebugCookieHeaders` dance with `std::fmt::FormatterFn` once it's stable:
	//
	// if tracing::enabled!(tracing::Level::DEBUG) {
	//     tracing::Span::current().record(
	//         "cookies",
	//         tracing::field::debug(FormatterFn(|f| {
	//             f.debug_list().entries(&cookie_headers).finish()
	//         }),
	//     );
	// }

	struct DebugCookieHeaders<'a>(&'a http::header::GetAll<'a, http::HeaderValue>);

	impl fmt::Debug for DebugCookieHeaders<'_> {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			f.debug_list().entries(self.0).finish()
		}
	}

	let cookie_headers = headers.get_all(http::header::COOKIE);

	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::Span::current().record(
			"cookies",
			tracing::field::debug(DebugCookieHeaders(&cookie_headers)),
		);
	}

	cookie_headers
		.into_iter()
		.flat_map(|v| v.to_str())
		.flat_map(|v| Cookie::split_parse_encoded(v.trim()))
		.flatten()
		.find(|cookie| cookie.name() == COOKIE_NAME)
		.map(|cookie| cookie.value().parse::<SessionID>())
		.ok_or(SessionRejection::MissingCookie)?
		.map_err(SessionRejection::ParseSessionID)
}
