use std::future::Future;

use cookie::Expiration;

use crate::{Session, SessionID};

/// A session store.
///
/// This is used by the [`SessionManager`] to retrieve / update sessions.
///
/// [`SessionManager`]: crate::SessionManager
pub trait SessionStore: Send + Sync + 'static
{
	/// The type to use for session IDs.
	type ID: SessionID;

	/// The session data.
	type Data: Send + Sync + 'static;

	/// An error type that can be returned from the methods.
	type Error: std::error::Error + Send + Sync + 'static;

	/// Loads a session from the store.
	///
	/// This is called right before the inner service, and the resulting [`Session`] will be
	/// inserted into the [request extensions].
	///
	/// [request extensions]: http::Request::extensions
	fn load_session(
		&mut self,
		session_id: &Self::ID,
	) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send;

	/// Saves a session to the store.
	///
	/// This is called right after the inner service, if [`Session::invalidate()`] was not
	/// called.
	///
	/// The returned [`Expiration`] will determine when the returned HTTP cookie will expire.
	fn save_session(
		&mut self,
		session: Session<Self::ID, Self::Data>,
	) -> impl Future<Output = Result<Expiration, Self::Error>> + Send;

	/// Invalidates a session in the store.
	///
	/// This is called right after the inner service, if [`Session::invalidate()`] was called.
	///
	/// The returned HTTP cookie will be invalidated by the middleware.
	fn invalidate_session(
		&mut self,
		session: Session<Self::ID, Self::Data>,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
