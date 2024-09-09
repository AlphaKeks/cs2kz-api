use std::fmt;

use thiserror::Error;
use tower_service::Service;

use crate::{AuthorizeSession, SessionID, SessionStore};

/// The error returned by the [`SessionManager`] middleware.
///
/// [`SessionManager`]: crate::SessionManager
#[derive(Error)]
pub enum SessionManagerError<Store, Auth, ReqBody, Inner>
where
	Store: SessionStore,
	<Store::ID as SessionID>::Error: std::error::Error,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	Inner: Service<http::Request<ReqBody>>,
	Inner::Error: std::error::Error + 'static,
{
	/// The session ID was not in the request cookies.
	#[error("missing session ID")]
	MissingSessionID,

	/// The session ID was found in the cookies, but could not be decoded.
	#[error("failed to decode session ID")]
	DecodeSessionID(#[source] <Store::ID as SessionID>::Error),

	/// The session ID we extracted from the request could not be found in the store.
	#[error("failed to authenticate session")]
	AuthenticateSession(#[source] Store::Error),

	/// The session could not be authorized.
	#[error("failed to authorize session")]
	AuthorizeSession(#[source] Auth::Error),

	/// The session could not be saved back to the store.
	#[error("failed to save session")]
	SaveSession(#[source] Store::Error),

	/// The session could not be invalidated by the store.
	#[error("failed to invalidate session")]
	InvalidateSession(#[source] Store::Error),

	/// The inner service returned an error.
	#[error("{0}")]
	Service(#[source] Inner::Error),
}

impl<Store, Auth, ReqBody, Inner> fmt::Debug for SessionManagerError<Store, Auth, ReqBody, Inner>
where
	Store: SessionStore,
	<Store::ID as SessionID>::Error: std::error::Error,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	Inner: Service<http::Request<ReqBody>>,
	Inner::Error: std::error::Error + 'static,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match self {
			Self::MissingSessionID => f.pad("MissingSessionID"),
			Self::DecodeSessionID(source) => fmt::Debug::fmt(source, f),
			Self::AuthenticateSession(source) => fmt::Debug::fmt(source, f),
			Self::AuthorizeSession(source) => fmt::Debug::fmt(source, f),
			Self::SaveSession(source) => fmt::Debug::fmt(source, f),
			Self::InvalidateSession(source) => fmt::Debug::fmt(source, f),
			Self::Service(source) => fmt::Debug::fmt(source, f),
		}
	}
}
