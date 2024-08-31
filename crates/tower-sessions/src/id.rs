use std::borrow::Cow;

/// A session ID.
pub trait SessionID: Sized + Send + Sync + 'static
{
	/// An error type to return from [`decode`].
	///
	/// [`decode`]: SessionID::decode()
	type Error: std::error::Error + Send + Sync + 'static;

	/// The name of the cookie the ID is stored in.
	fn cookie_name() -> Cow<'static, str>;

	/// Encodes a session ID into a string so it can be stored in an HTTP cookie.
	fn encode(&self) -> Cow<'static, str>;

	/// Decodes a session ID from a cookie value.
	fn decode(s: &str) -> Result<Self, Self::Error>;
}
