use std::sync::atomic::{self, AtomicBool};
use std::sync::Arc;

/// A session.
#[derive(Clone)]
pub struct Session<ID, Data>
{
	/// The session ID.
	id: ID,

	/// The session data.
	data: Data,

	/// The state of the session.
	state: State,

	/// Whether the session was invalidated.
	invalidated: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd)]
enum State
{
	NotAuthenticated,
	Authenticated,
	Authorized,
}

impl<ID, Data> Session<ID, Data>
{
	/// Creates a new [`Session`].
	pub(crate) fn new(id: ID, data: Data) -> Self
	{
		Self {
			id,
			data,
			state: State::NotAuthenticated,
			invalidated: Arc::new(AtomicBool::new(false)),
		}
	}

	/// Returns the session ID.
	pub fn id(&self) -> &ID
	{
		&self.id
	}

	/// Returns the session data.
	pub fn data(&self) -> &Data
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
