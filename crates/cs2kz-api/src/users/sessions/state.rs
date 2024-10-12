use std::fmt;
use std::sync::atomic::{self, AtomicI8};

// The performance of the atomic operations in this module doesn't matter much in practice, so
// better safe than sorry.
const ORDERING: atomic::Ordering = atomic::Ordering::SeqCst;

/// The session has been explicitly invalidated.
const INVALIDATED: i8 = -1;

/// The initial state.
const AUTHENTICATED: i8 = 0;

/// The authorization requirements for the request have been satisfied.
const AUTHORIZED: i8 = 1;

/// Session state.
///
/// While the authentication middleware runs, the constructed [`Session`] object goes through
/// multiple states:
///
///    1. "authenticated" -> the initial state
///    2. "authorized" -> after the authorization checks for the request have passed
///    3. "invalidated" -> after the request handler, if [`Session::invalidate()`] was called
///
/// After the request handler returns, the state decides how the session is persisted.
pub(super) struct State(AtomicI8);

impl State {
	pub(super) fn authenticated() -> Self {
		Self(AtomicI8::new(AUTHENTICATED))
	}

	pub(super) fn authorized() -> Self {
		Self(AtomicI8::new(AUTHORIZED))
	}

	pub(super) fn invalidated() -> Self {
		Self(AtomicI8::new(INVALIDATED))
	}

	pub(super) fn is_authorized(&self) -> bool {
		self.0.load(ORDERING) == AUTHORIZED
	}

	pub(super) fn is_invalidated(&self) -> bool {
		self.0.load(ORDERING) == INVALIDATED
	}

	pub(super) fn set(&self, state: State) {
		self.0.store(state.0.into_inner(), ORDERING);
	}
}

impl fmt::Debug for State {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self.0.load(ORDERING) {
			INVALIDATED => "invalidated",
			AUTHENTICATED => "authenticated",
			AUTHORIZED => "authorized",
			state => unreachable!("invalid session state: {state}"),
		})
	}
}
