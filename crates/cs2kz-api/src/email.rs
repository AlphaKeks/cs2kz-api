use std::str::FromStr;

use ::email_address as base;

/// An email address.
///
/// The primary way to obtain a value of this type is via [`FromStr`] / [`serde::Deserialize`].
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct EmailAddress(base::EmailAddress);

#[derive(Debug, Error)]
#[error("failed to parse email address: {0}")]
pub struct ParseEmailError(#[from] base::Error);

impl EmailAddress {
	pub fn as_str(&self) -> &str {
		self.0.as_str()
	}
}

impl FromStr for EmailAddress {
	type Err = ParseEmailError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		s.parse::<base::EmailAddress>()
			.map(Self)
			.map_err(ParseEmailError)
	}
}

sqlx_type!(EmailAddress as str => {
	get: |self| self.as_str();
	make: |value| Ok(value.parse::<Self>()?);
});
