use uuid::Uuid;

/// A session ID.
#[derive(Debug, Clone, Copy, PartialEq, FromStr, serde::Serialize, serde::Deserialize)]
#[debug("{}", _0.as_hyphenated())]
pub struct SessionID(Uuid);

impl SessionID {
	/// Generates a new session ID.
	pub fn new() -> Self {
		Self(Uuid::new_v4())
	}
}

impl Default for SessionID {
	/// The default value for session IDs is `00000000-0000-0000-0000-000000000000`.
	fn default() -> Self {
		Self(Uuid::nil())
	}
}

sqlx_type!(SessionID as [u8] => {
	get: |self| &self.0.as_bytes()[..];
	make: |bytes| Ok(Self(Uuid::from_slice(bytes)?));
});
