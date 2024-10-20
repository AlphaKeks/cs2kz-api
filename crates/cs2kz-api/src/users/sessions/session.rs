use std::fmt;
use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;

use axum::extract::{FromRef, FromRequestParts};
use axum::response::{IntoResponse, Response};
use axum_extra::extract::cookie::{Cookie, SameSite};
use problem_details::AsProblemDetails;

use super::SessionID;
use crate::config::CookieConfig;
use crate::database::{self, DatabaseError};
use crate::http::problem_details::Problem;
use crate::users::permissions::Permissions;
use crate::users::{sessions, UserID};

/// The name of the cookie holding the session ID.
const COOKIE_NAME: &str = "kz-auth";
const ERROR_MESSAGE: &str = "you are not permitted to perform this action";

/// A user session.
///
/// This type contains information about the current session and its associated user.
/// It acts as an [extractor], and is commonly used with the [`session_auth()`] middleware.
///
/// [extractor]: axum::extract
/// [`session_auth()`]: super::http::middleware::session_auth
#[derive(Clone)]
pub struct Session(Arc<SessionInner>);

#[derive(Debug)]
struct SessionInner {
	/// The session's ID.
	id: SessionID,

	/// Information about the user this session belongs to.
	user: UserInfo,

	/// Whether the session is still valid.
	///
	/// This is true by default, but can be changed with [`Session::invalidate()`].
	/// The value of this field determines whether the session is extended or expired by the
	/// middleware after the handler has returned.
	is_valid: AtomicBool,
}

/// Information about a logged-in user.
#[derive(Debug, Clone)]
pub struct UserInfo {
	/// The user's ID.
	id: UserID,

	/// The user's permissions at the time of the request.
	permissions: Permissions,
}

#[derive(Debug, Error)]
pub enum SessionRejection {
	#[error("{ERROR_MESSAGE}")]
	MissingCookie,

	#[error("{ERROR_MESSAGE}")]
	ParseSessionID(#[from] uuid::Error),

	#[error("{ERROR_MESSAGE}")]
	InvalidSessionID,

	#[error("something went wrong; please report this incident")]
	Database(#[from] DatabaseError),
}

impl Session {
	fn new(id: SessionID, user: UserInfo) -> Self {
		Self(Arc::new(SessionInner {
			id,
			user,
			is_valid: AtomicBool::new(true),
		}))
	}

	/// Returns the session's ID.
	pub fn id(&self) -> SessionID {
		self.0.id
	}

	/// Returns information about the user associated with the session.
	pub fn user(&self) -> &UserInfo {
		&self.0.user
	}

	/// Checks if the session is still valid.
	pub fn is_valid(&self) -> bool {
		self.0.is_valid.load(atomic::Ordering::SeqCst)
	}

	/// Invalidates this session.
	///
	/// This function will return `true` if the session was valid before.
	pub fn invalidate(&self) -> bool {
		self.0.is_valid.swap(false, atomic::Ordering::SeqCst)
	}

	/// Creates an HTTP [`Cookie`] containing the session ID.
	pub(super) fn as_cookie(&self, config: &CookieConfig) -> Cookie<'static> {
		config
			.build_cookie(COOKIE_NAME, format!("{:?}", self.id()))
			.same_site(SameSite::Strict)
			.http_only(true)
			.build()
	}
}

impl fmt::Debug for Session {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&*self.0, f)
	}
}

impl From<sessions::database::Session> for Session {
	fn from(session: sessions::database::Session) -> Self {
		Self::new(session.id, UserInfo {
			id: session.user_id,
			permissions: session.user_permissions,
		})
	}
}

impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync,
	database::ConnectionPool: FromRef<S>,
{
	type Rejection = SessionRejection;

	#[instrument(
		level = "debug",
		skip_all,
		ret(level = "debug"),
		err(Debug, level = "debug")
	)]
	async fn from_request_parts(
		request: &mut http::request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		if let Some(cached) = request.extensions.get::<Session>().cloned() {
			debug!("returning cached session");
			return Ok(cached);
		}

		let mut conn = database::ConnectionPool::from_ref(state)
			.get_connection()
			.await?;

		let session_id = extract_session_id(&request.headers)?;
		let session = match sessions::database::find(&mut conn, session_id).await {
			Ok(Some(session)) => Session::from(session),
			Ok(None) => return Err(SessionRejection::InvalidSessionID),
			Err(error) => return Err(error.into()),
		};

		Ok(session)
	}
}

impl UserInfo {
	pub fn id(&self) -> UserID {
		self.id
	}

	pub fn permissions(&self) -> Permissions {
		self.permissions
	}
}

impl AsProblemDetails for SessionRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::Unauthorized
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		if cfg!(feature = "production") {
			return;
		}

		match self {
			Self::MissingCookie | Self::InvalidSessionID => {}
			Self::ParseSessionID(source) => {
				_ = extension_members.add("parse_error", &format_args!("{source}"));
			}
			Self::Database(source) => {
				_ = extension_members.add("database_error", &format_args!("{source}"));
			}
		}
	}
}

impl IntoResponse for SessionRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

/// Extracts a [`SessionID`] from the given headers.
#[instrument(level = "trace", skip_all, fields(cookies = tracing::field::Empty), ret(level = "trace"), err(Debug, level = "debug"))]
fn extract_session_id(headers: &http::HeaderMap) -> Result<SessionID, SessionRejection> {
	/// A helper type for debug-formatting `Cookie` header values.
	///
	/// NOTE: If `std::fmt::FormatterFn` is ever stabilized, use it instead.
	struct DebugCookieHeaders<'a>(&'a http::header::GetAll<'a, http::HeaderValue>);

	impl fmt::Debug for DebugCookieHeaders<'_> {
		fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
			f.debug_list().entries(self.0).finish()
		}
	}

	let cookie_headers = headers.get_all(http::header::COOKIE);

	if tracing::enabled!(tracing::Level::DEBUG) {
		tracing::Span::current()
			.record("cookies", tracing::field::debug(DebugCookieHeaders(&cookie_headers)));
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
