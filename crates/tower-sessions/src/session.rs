use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;

use crate::SessionStore;

/// A session.
#[derive(Debug)]
pub struct Session<S>
where
	S: SessionStore + ?Sized,
{
	/// The session ID.
	id: S::ID,

	/// The session data.
	data: S::Data,

	/// The state of the session.
	state: State,

	/// Whether the session was invalidated.
	invalidated: Arc<AtomicBool>,
}

impl<S> Clone for Session<S>
where
	S: SessionStore + ?Sized,
	S::ID: Clone,
	S::Data: Clone,
{
	fn clone(&self) -> Self
	{
		Self {
			id: self.id.clone(),
			data: self.data.clone(),
			state: self.state,
			invalidated: Arc::clone(&self.invalidated),
		}
	}

	fn clone_from(&mut self, source: &Self)
	{
		self.id.clone_from(&source.id);
		self.data.clone_from(&source.data);
		self.state = source.state;
		self.invalidated.clone_from(&source.invalidated);
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
enum State
{
	NotAuthenticated,
	Authenticated,
	Authorized,
}

impl<S> Session<S>
where
	S: SessionStore + ?Sized,
{
	/// Creates a new [`Session`].
	pub(crate) fn new(id: S::ID, data: S::Data) -> Self
	{
		Self {
			id,
			data,
			state: State::NotAuthenticated,
			invalidated: Arc::new(AtomicBool::new(false)),
		}
	}

	/// Returns the session ID.
	pub fn id(&self) -> &S::ID
	{
		&self.id
	}

	/// Returns the session data.
	pub fn data(&self) -> &S::Data
	{
		&self.data
	}

	/// Whether the session was authenticated.
	pub fn is_authenticated(&self) -> bool
	{
		self.state >= State::Authenticated
	}

	/// Whether the session was authorized.
	pub fn is_authorized(&self) -> bool
	{
		self.state == State::Authorized
	}

	/// Whether the session is valid.
	pub fn is_valid(&self) -> bool
	{
		!self.invalidated.load(atomic::Ordering::Acquire)
	}

	/// Invalidates this session.
	///
	/// This won't have any effect immediately, but it will ensure that
	/// [`SessionStore::invalidate_session()`] is called after your service is done.
	///
	/// [`SessionStore::invalidate_session()`]: crate::SessionStore::invalidate_session()
	pub fn invalidate(&self)
	{
		self.invalidated.store(true, atomic::Ordering::Release);
	}

	pub(crate) fn authenticate(&mut self)
	{
		self.state = State::Authenticated;
	}

	pub(crate) fn authorize(&mut self)
	{
		self.state = State::Authorized;
	}
}
