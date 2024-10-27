use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

/// An ID uniquely identifying a user session.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SessionID(Uuid);

/// Error returned by [`SessionID`]'s [`FromStr`] implementtion.
#[derive(Debug, Error)]
#[error("failed to parse session ID: {0}")]
pub struct ParseSessionIDError(#[from] uuid::Error);

impl SessionID {
	/// Generates a new session ID.
	#[expect(
		clippy::new_without_default,
		reason = "as session IDs are generated randomly, there is no good 'default' to choose"
	)]
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl fmt::Debug for SessionID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_tuple("SessionID")
			.field(self.0.as_hyphenated())
			.finish()
	}
}

impl fmt::Display for SessionID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self.0.as_hyphenated(), fmt)
	}
}

impl FromStr for SessionID {
	type Err = ParseSessionIDError;

	fn from_str(str: &str) -> Result<Self, Self::Err> {
		str.parse::<Uuid>().map(Self).map_err(ParseSessionIDError)
	}
}

crate::database::uuid_as_bytes!(SessionID);
