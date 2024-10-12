use std::borrow::Cow;

use crate::database;

#[derive(Debug, Display, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub struct GitRevision(String);

impl GitRevision {
	/// Creates a new [`GitRevision`].
	///
	/// If the provided `revision` does not contain exactly 40 ASCII characters, this function
	/// returns [`None`].
	pub fn new(revision: impl Into<String>) -> Option<Self> {
		let revision: String = revision.into();

		if revision.len() != 40 || !revision.is_ascii() {
			return None;
		}

		Some(Self(revision))
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl<'de> serde::Deserialize<'de> for GitRevision {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		let value = String::deserialize(deserializer)?;

		if value.len() != 40 {
			return Err(de::Error::invalid_length(value.len(), &"40"));
		}

		if !value.is_ascii() {
			return Err(de::Error::invalid_value(
				de::Unexpected::Str(&value),
				&"a 40-character ASCII string",
			));
		}

		Ok(Self(value))
	}
}

database::macros::wrap!(GitRevision as str => {
	get: |self| self.as_str();
	make: |value| GitRevision::new(value).ok_or_else(|| "invalid git revision".into());
});
