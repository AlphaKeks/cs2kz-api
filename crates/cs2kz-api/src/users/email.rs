use std::str::FromStr;

mod base {
	pub(super) use email_address::{EmailAddress as Email, Error};
}

/// An email address.
///
/// The primary way to obtain a value of this type is via [`FromStr`] / [`serde::Deserialize`].
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Email(base::Email);

#[derive(Debug, Error)]
#[error("failed to parse email address: {0}")]
pub struct ParseEmailError(#[from] base::Error);

impl Email {
	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}
}

impl FromStr for Email {
	type Err = ParseEmailError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.parse::<base::Email>().map(Self).map_err(ParseEmailError)
	}
}
