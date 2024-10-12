use uuid::Uuid;

use crate::database;

#[derive(Debug, Clone, Copy, PartialEq, FromStr, serde::Serialize, serde::Deserialize)]
#[debug("{}", _0.as_hyphenated())]
pub struct AccessKey(Uuid);

impl AccessKey {
	pub(in crate::servers) fn new() -> Self {
		Self(Uuid::new_v4())
	}

	/// Generates an "invalid" access key.
	///
	/// This is a sentinel value used in the database instead of `NULL`.
	pub(in crate::servers) fn invalid() -> Self {
		Self(Uuid::nil())
	}
}

database::macros::wrap!(AccessKey as [u8] => {
	get: |self| &self.0.as_bytes()[..];
	make: |bytes| Ok(Self(Uuid::from_slice(bytes)?));
});
