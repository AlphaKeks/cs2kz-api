//! Session authorization.

use std::any::type_name;
use std::convert::Infallible;
use std::fmt;
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
		session: &Session<Self::Store>,
		request: &mut http::Request<Self::ReqBody>,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;

	/// Combines `self` with a fallback strategy.
	///
	/// See the [`Or`] docs for more details.
	fn or<B>(self, fallback: B) -> Or<Self::ReqBody, Self::Store, Self, B>
	where
		Self: Sized,
		B: AuthorizeSession<ReqBody = Self::ReqBody, Store = Self::Store>,
	{
		Or {
			a: self,
			b: fallback,
			_marker: PhantomData,
		}
	}
}

/// No authorization.
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
		_: &Session<Self::Store>,
		_: &mut http::Request<Self::ReqBody>,
	) -> Result<(), Self::Error>
	{
		Ok(())
	}
}

impl<Body, Store> fmt::Debug for None<Body, Store>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		write!(f, "None<{}, {}>", type_name::<Body>(), type_name::<Store>())
	}
}

impl<Body, Store> Clone for None<Body, Store>
{
	fn clone(&self) -> Self
	{
		*self
	}
}

impl<Body, Store> Copy for None<Body, Store> {}

/// Authorize either using method `A`, or try `B` as a fallback.
pub struct Or<Body, Store, A, B>
{
	a: A,
	b: B,
	_marker: PhantomData<fn() -> (Body, Store)>,
}

impl<Body, Store, A, B> AuthorizeSession for Or<Body, Store, A, B>
where
	Body: Send + 'static,
	Store: SessionStore,
	A: AuthorizeSession<ReqBody = Body, Store = Store>,
	B: AuthorizeSession<ReqBody = Body, Store = Store>,
{
	type ReqBody = Body;
	type Store = Store;
	type Error = B::Error;

	async fn authorize_session(
		&mut self,
		session: &Session<Self::Store>,
		request: &mut http::Request<Self::ReqBody>,
	) -> Result<(), Self::Error>
	{
		if self.a.authorize_session(session, request).await.is_ok() {
			return Ok(());
		}

		self.b.authorize_session(session, request).await
	}
}

impl<Body, Store, A, B> fmt::Debug for Or<Body, Store, A, B>
where
	A: fmt::Debug,
	B: fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct(&format!(
			"Or<{}, {}>",
			type_name::<Body>(),
			type_name::<Store>()
		))
		.field("a", &self.a)
		.field("b", &self.b)
		.finish()
	}
}

impl<Body, Store, A, B> Clone for Or<Body, Store, A, B>
where
	A: Clone,
	B: Clone,
{
	fn clone(&self) -> Self
	{
		Self {
			a: self.a.clone(),
			b: self.b.clone(),
			_marker: PhantomData,
		}
	}

	fn clone_from(&mut self, source: &Self)
	{
		self.a.clone_from(&source.a);
		self.b.clone_from(&source.b);
	}
}
