//! Session authorization.

use std::convert::Infallible;
use std::future::Future;
use std::marker::PhantomData;

use crate::{Session, SessionStore};

/// How to authorize a session.
pub trait AuthorizeSession: Send + Sync + 'static
{
	/// The HTTP request body.
	type ReqBody;

	/// The session store.
	type Store: SessionStore;

	/// An error that can be returned from [`authorize_session()`].
	///
	/// [`authorize_session()`]: AuthorizeSession::authorize_session()
	type Error: std::error::Error + Send + Sync + 'static;

	/// Authorizes a session.
	fn authorize_session(
		&mut self,
		session: &Session<<Self::Store as SessionStore>::ID, <Self::Store as SessionStore>::Data>,
		request: &mut http::Request<Self::ReqBody>,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// No authorization.
#[derive(Debug, Clone, Copy)]
pub struct None<Body, Store>(PhantomData<fn() -> (Body, Store)>);

impl<Body, Store> None<Body, Store>
{
	/// Creates a new [`None`].
	pub const fn new() -> Self
	{
		Self(PhantomData)
	}
}

impl<Body, Store> AuthorizeSession for None<Body, Store>
where
	Body: Send + 'static,
	Store: SessionStore,
{
	type ReqBody = Body;
	type Store = Store;
	type Error = Infallible;

	async fn authorize_session(
		&mut self,
		_: &Session<<Self::Store as SessionStore>::ID, <Self::Store as SessionStore>::Data>,
		_: &mut http::Request<Self::ReqBody>,
	) -> Result<(), Self::Error>
	{
		Ok(())
	}
}
