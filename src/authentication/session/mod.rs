//! Session Authentication.

use std::marker::PhantomData;

use axum::extract::FromRequestParts;
use axum::http::request;
use axum::response::{IntoResponseParts, ResponseParts};
use axum::{async_trait, http};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use derive_more::{Debug, Display, Into};
use reqwest::header;
use sqlx::{MySql, Transaction};
use time::OffsetDateTime;
use tracing::{debug, trace};

use crate::authentication::User;
use crate::authorization::{self, AuthorizeError, AuthorizeSession, Permissions};
use crate::database::SqlxErrorExt;
use crate::{Config, State};

mod id;

#[doc(inline)]
pub use id::SessionID;

mod error;

#[doc(inline)]
pub use error::AuthenticateSessionError;

/// The cookie name used for storing session tokens.
pub const COOKIE_NAME: &str = "kz-auth";

/// A session.
#[derive(Debug, Into)]
pub struct Session<A = authorization::None> {
	/// The session ID.
	id: SessionID,

	/// The logged-in user.
	#[debug("{}", user.steam_id())]
	user: User,

	/// The cookie that will be put in the user's browser.
	#[debug(skip)]
	#[into]
	cookie: Cookie<'static>,

	/// Marker for authorization method.
	#[debug(skip)]
	_auth: PhantomData<A>,
}

impl<A> Session<A> {
	/// Returns the session ID.
	pub const fn id(&self) -> SessionID {
		self.id
	}

	/// Returns the logged-in user.
	pub const fn user(&self) -> User {
		self.user
	}

	/// Returns the expiration date for a new [Session].
	fn expires_on() -> OffsetDateTime {
		OffsetDateTime::now_utc() + time::Duration::WEEK
	}
}

impl Session {
	/// Create a new [Session].
	///
	/// This will insert a session into the database and prepare a cookie to send to
	/// the user. [`Session`] implements [`IntoResponseParts`], the caller of this
	/// function should return it from a handler / middleware.
	pub async fn create(
		user_id: SteamID,
		config: &Config,
		transaction: &mut Transaction<'static, MySql>,
	) -> Result<Self, AuthenticateSessionError> {
		let session_id = SessionID::new();
		let expires_on = Self::expires_on();

		sqlx::query! {
			r#"
			INSERT INTO
			  LoginSessions (id, user_id, expires_on)
			VALUES
			  (?, ?, ?)
			"#,
			session_id,
			user_id,
			expires_on,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation("user_id") {
				AuthenticateSessionError::UnknownUser { source: err }
			} else {
				AuthenticateSessionError::from(err)
			}
		})?;

		let user = sqlx::query_scalar! {
			r#"
			SELECT
			  permissions `permissions: Permissions`
			FROM
			  Players
			WHERE
			  id = ?
			"#,
			user_id,
		}
		.fetch_one(transaction.as_mut())
		.await
		.map(|permissions| User::new(user_id, permissions))?;

		debug!(%session_id, %user_id, "authenticated user");

		let cookie = Cookie::build((COOKIE_NAME, session_id.to_string()))
			.domain(config.cookie_domain.clone())
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(true)
			.expires(expires_on)
			.build();

		Ok(Self {
			id: session_id,
			user,
			cookie,
			_auth: PhantomData,
		})
	}

	/// Invalidate this session.
	///
	/// Note that in order for the invalidation to propagate to the user, `self` must be
	/// returned from an HTTP handler / middleware so the [`IntoResponseParts`] implementation
	/// kicks in.
	pub async fn invalidate(
		&mut self,
		invalidate: InvalidateSessions,
		transaction: &mut Transaction<'static, MySql>,
	) -> sqlx::Result<()> {
		sqlx::query! {
			r#"
			UPDATE
			  LoginSessions
			SET
			  expires_on = NOW()
			WHERE
			  user_id = ?
			  AND expires_on > NOW()
			  AND (
			    id = ?
			    OR ?
			  )
			"#,
			self.user.steam_id(),
			self.id,
			invalidate == InvalidateSessions::All,
		}
		.execute(transaction.as_mut())
		.await?;

		debug! {
			session.id = %self.id,
			user.id = %self.user.steam_id(),
			all = %invalidate,
			"invalidated session(s) for user"
		};

		self.cookie.set_expires(OffsetDateTime::now_utc());

		Ok(())
	}
}

#[async_trait]
impl<A> FromRequestParts<&'static State> for Session<A>
where
	A: AuthorizeSession,
{
	type Rejection = AuthorizeError;

	async fn from_request_parts(
		req: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self, Self::Rejection> {
		if let Some(session) = req.extensions.remove::<Self>() {
			debug!(%session.id, "extracted cached session");
			return Ok(session);
		}

		let (mut cookie, session_id) = req
			.headers
			.get_all(header::COOKIE)
			.into_iter()
			.flat_map(|value| value.to_str())
			.flat_map(|value| Cookie::split_parse_encoded(value.trim().to_owned()))
			.flatten()
			.find_map(|cookie| {
				if cookie.name() != COOKIE_NAME {
					return None;
				}

				cookie
					.value()
					.parse::<SessionID>()
					.map(|session_id| Some((cookie, session_id)))
					.map_err(AuthorizeError::InvalidSessionID)
					.transpose()
			})
			.ok_or(AuthorizeError::MissingSessionID)??;

		let mut transaction = state.database.begin().await?;

		let session = sqlx::query! {
			r#"
			SELECT
			  s.id `id: SessionID`,
			  p.id `user_id: SteamID`,
			  p.permissions `permissions: Permissions`
			FROM
			  LoginSessions s
			  JOIN Players p ON p.id = s.user_id
			WHERE
			  s.id = ?
			  AND s.expires_on > NOW()
			ORDER BY
			  expires_on DESC
			LIMIT
			  1
			"#,
			session_id,
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.ok_or(AuthorizeError::InvalidSession)?;

		trace!(%session.id, "authenticated session");

		let expires_on = Self::expires_on();

		cookie.set_path("/");
		cookie.set_secure(cfg!(feature = "production"));
		cookie.set_http_only(true);
		cookie.set_expires(expires_on);

		sqlx::query! {
			r#"
			UPDATE
			  LoginSessions
			SET
			  expires_on = ?
			WHERE
			  id = ?
			"#,
			expires_on,
			session.id,
		}
		.execute(transaction.as_mut())
		.await?;

		trace!(%session.id, "extended session");

		let user = User::new(session.user_id, session.permissions);

		A::authorize_session(&user, req, &mut transaction).await?;

		transaction.commit().await?;

		Ok(Self {
			id: session.id,
			user,
			cookie,
			_auth: PhantomData,
		})
	}
}

impl<A> IntoResponseParts for Session<A>
where
	A: AuthorizeSession,
{
	type Error = AuthorizeError;

	fn into_response_parts(self, mut res: ResponseParts) -> Result<ResponseParts, Self::Error> {
		let cookie = Cookie::from(self)
			.encoded()
			.to_string()
			.parse::<http::HeaderValue>()
			.expect("valid cookie");

		res.headers_mut().insert(header::SET_COOKIE, cookie);

		Ok(res)
	}
}

/// An enum describing which sessions to invalidate.
///
/// See [`Session::invalidate()`].
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq)]
pub enum InvalidateSessions {
	/// Only invalidate the current session.
	#[display("current")]
	Current,

	/// Invalidate all sessions that are still active.
	#[display("all")]
	All,
}

impl<A> Clone for Session<A> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			user: self.user,
			cookie: self.cookie.clone(),
			_auth: PhantomData,
		}
	}
}
