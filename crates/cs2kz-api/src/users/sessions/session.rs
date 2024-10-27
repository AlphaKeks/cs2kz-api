use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{self, AtomicBool};

use axum::extract::{FromRef, FromRequestParts};
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use problem_details::AsProblemDetails;

use crate::config::CookieConfig;
use crate::database::{self, DatabaseError};
use crate::http::problem_details::Problem;
use crate::users::sessions::{self, ParseSessionIDError, SessionID};
use crate::users::{Permissions, UserID};

const COOKIE_NAME: &str = "kz-auth";

/// An authenticated session.
///
/// This object is created by the [`session_auth`] middleware and inserted into
/// the [request extensions]. It also acts as an [extractor] by itself. If the
/// middleware is used, sessions can be invalidated by calling
/// [`Session::invalidate()`]. This will not do anything immediately, but
/// after the service wrapped by `session_auth` returned. If the session is not
/// invalidated, it is extended instead.
///
/// [`session_auth`]: crate::http::middleware::session_auth
/// [request extensions]: http::Request::extensions
/// [extractor]: axum::extract
#[derive(Clone)]
pub struct Session(Arc<Inner>);

/// Information about a logged-in user.
#[derive(Debug)]
pub struct UserInfo {
	id: UserID,
	permissions: Permissions,
}

/// Rejections for the [`Session`] extractor.
#[derive(Debug, Error)]
#[expect(
	missing_docs,
	reason = "variant names + error messages should be self-documenting"
)]
pub enum SessionRejection {
	#[cfg_attr(not(feature = "production"), error("missing session cookie"))]
	#[cfg_attr(
		feature = "production",
		error("you are not permitted to make this request")
	)]
	MissingCookie,

	#[cfg_attr(not(feature = "production"), error(transparent))]
	#[cfg_attr(
		feature = "production",
		error("you are not permitted to make this request")
	)]
	ParseCookieValue(#[from] ParseSessionIDError),

	#[cfg_attr(not(feature = "production"), error("invalid session ID"))]
	#[cfg_attr(
		feature = "production",
		error("you are not permitted to make this request")
	)]
	InvalidSessionID,

	#[cfg_attr(not(feature = "production"), error(transparent))]
	#[cfg_attr(
		feature = "production",
		error("you are not permitted to make this request")
	)]
	Database(#[from] DatabaseError),
}

struct Inner {
	id: SessionID,
	user: UserInfo,
	is_valid: AtomicBool,
}

impl Session {
	pub(crate) fn new(id: SessionID, user: UserInfo) -> Self {
		Self(Arc::new(Inner {
			id,
			user,
			is_valid: AtomicBool::new(true),
		}))
	}

	/// Returns the session's ID.
	pub fn id(&self) -> SessionID {
		self.0.id
	}

	/// Returns the user associated with the session.
	pub fn user(&self) -> &UserInfo {
		&self.0.user
	}

	/// Returns whether the session is still "valid".
	///
	/// This is the default state, but can be changed by calling
	/// [`Session::invalidate()`].
	pub fn is_valid(&self) -> bool {
		self.0.is_valid.load(atomic::Ordering::SeqCst)
	}

	/// Invalidates the session.
	///
	/// This will cause [`Session::is_valid()`] to return `false`.
	///
	/// The return value is the **previous** "valid" status.
	pub fn invalidate(&self) -> bool {
		self.0.is_valid.swap(false, atomic::Ordering::SeqCst)
	}

	pub(crate) fn as_cookie(&self, config: &CookieConfig) -> Cookie<'static> {
		let mut cookie = config
			.build_cookie(COOKIE_NAME, self.id().to_string())
			.build();

		if !self.is_valid() {
			cookie.make_removal();
		}

		cookie
	}
}

impl UserInfo {
	pub(crate) fn new(id: UserID, permissions: Permissions) -> Self {
		Self { id, permissions }
	}

	/// Returns the user's ID.
	pub fn id(&self) -> UserID {
		self.id
	}

	/// Returns the user's permissions.
	pub fn permissions(&self) -> Permissions {
		self.permissions
	}
}

impl fmt::Debug for Session {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_struct("Session")
			.field("id", &self.0.id)
			.field("user", &self.0.user)
			.field("is_valid", &self.0.is_valid)
			.finish()
	}
}

impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync,
	database::ConnectionPool: FromRef<S>,
{
	type Rejection = SessionRejection;

	async fn from_request_parts(
		parts: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		if let Some(cached) = parts.extensions.get::<Self>().cloned() {
			trace!(id = ?cached.id(), "extraced cached session");
			return Ok(cached);
		}

		let session_id = extract_session_id(&parts.headers)?;
		let mut conn = database::ConnectionPool::from_ref(state)
			.get_connection()
			.await?;

		let user_info = sessions::get_user_info(&mut conn, session_id)
			.await?
			.ok_or(SessionRejection::InvalidSessionID)?;

		let session = Self::new(session_id, user_info);

		parts.extensions.insert(session.clone());

		Ok(session)
	}
}

impl AsProblemDetails for SessionRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		match self {
			Self::MissingCookie | Self::ParseCookieValue(_) | Self::InvalidSessionID => {
				Problem::Unauthorized
			},
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl IntoResponse for SessionRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

fn extract_session_id(headers: &http::HeaderMap) -> Result<SessionID, SessionRejection> {
	headers
		.get_all(http::header::COOKIE)
		.into_iter()
		.flat_map(|value| value.to_str())
		.flat_map(|value| Cookie::split_parse_encoded(value.trim()))
		.flatten()
		.find(|cookie| cookie.name() == COOKIE_NAME)
		.ok_or(SessionRejection::MissingCookie)?
		.value()
		.parse::<SessionID>()
		.map_err(Into::into)
}
