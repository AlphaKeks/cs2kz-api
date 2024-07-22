//! Session authentication.
//!
//! For a quick overview, see the [`auth` top-level documentation].
//!
//! [`auth` top-level documentation]: crate::services::auth

use std::fmt;

use axum::async_trait;
use axum::extract::{FromRef, FromRequestParts};
use axum::http::{header, request};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use sqlx::{MySql, Pool};

use crate::runtime;

mod id;
pub use id::SessionID;

pub mod user;
pub use user::User;

mod rejection;
pub use rejection::SessionRejection;

pub mod authorization;
pub use authorization::AuthorizeSession;

mod service;
pub use service::{SessionManager, SessionManagerLayer};

/// The name of the HTTP cookie that will store the user's [session ID].
///
/// [session ID]: SessionID
pub const COOKIE_NAME: &str = "kz-auth";

/// An authenticated session.
///
/// This struct represents a session that has either just been created, or
/// extracted from a request.
#[must_use]
#[derive(Clone)]
pub struct Session
{
	/// The session's ID.
	id: SessionID,

	/// The user associated with this session.
	user: User,
}

impl Session
{
	/// Creates a new [`Session`].
	pub(super) fn new(id: SessionID, user: User) -> Self
	{
		Self { id, user }
	}

	/// Returns this session's ID.
	pub fn id(&self) -> SessionID
	{
		self.id
	}

	/// Returns the user associated with this session.
	pub fn user(&self) -> User
	{
		self.user
	}

	/// Creates an HTTP cookie from this session.
	pub fn into_cookie(self, api_config: &runtime::Config) -> Cookie<'static>
	{
		Cookie::build((COOKIE_NAME, self.id().to_string()))
			.domain(api_config.cookie_domain().to_owned())
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(true)
			.expires(super::generate_session_expiration_date())
			.build()
	}
}

impl fmt::Debug for Session
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Session")
			.field("id", &format_args!("{}", self.id))
			.field("user", &self.user)
			.finish()
	}
}

#[async_trait]
impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync + 'static,
	Pool<MySql>: FromRef<S>,
{
	type Rejection = SessionRejection;

	async fn from_request_parts(
		req: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		if let Some(session) = req.extensions.remove::<Self>() {
			return Ok(session);
		}

		let session_id = req
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| Cookie::split_parse_encoded(value.trim().to_owned()))
			.flatten()
			.find(|cookie| cookie.name() == COOKIE_NAME)
			.map(|cookie| cookie.value().parse::<SessionID>())
			.ok_or(SessionRejection::MissingCookie)??;

		let database = Pool::<MySql>::from_ref(state);
		let session = sqlx::query! {
			r"
			SELECT
			  u.id `user_id: SteamID`,
			  u.permissions `user_permissions: user::Permissions`
			FROM
			  LoginSessions s
			  JOIN Players u ON u.id = s.player_id
			WHERE
			  s.id = ?
			  AND s.expires_on > NOW()
			ORDER BY
			  expires_on DESC
			",
			session_id,
		}
		.fetch_optional(&database)
		.await?
		.map(|row| Session::new(session_id, User::new(row.user_id, row.user_permissions)))
		.ok_or(SessionRejection::InvalidSessionID)?;

		tracing::trace! {
			session.id = %session.id(),
			user.id = %session.user().steam_id(),
			"authenticated session",
		};

		Ok(session)
	}
}
