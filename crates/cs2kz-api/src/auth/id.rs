use std::borrow::Cow;
use std::fmt;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionID(Uuid);

impl SessionID
{
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}
}

impl fmt::Display for SessionID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(self.0.as_hyphenated(), f)
	}
}

#[derive(Debug, Error)]
#[error("failed to decode session ID: {0}")]
pub struct DecodeSessionID(#[from] uuid::Error);

impl tower_sessions::SessionID for SessionID
{
	type Error = DecodeSessionID;

	fn cookie_name() -> Cow<'static, str>
	{
		Cow::Borrowed("kz-auth")
	}

	fn encode(&self) -> Cow<'static, str>
	{
		Cow::Owned(self.0.as_hyphenated().to_string())
	}

	fn decode(s: &str) -> Result<Self, Self::Error>
	{
		s.parse::<Uuid>().map(Self).map_err(Into::into)
	}
}

sql_type!(SessionID as Uuid => {
	encode_by_ref: |self| &self.0,
	encode: |self| self.0,
	decode: |value| Ok(Self(value)),
});
