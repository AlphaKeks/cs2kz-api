use std::ops;

use serde::{Deserialize, Serialize};

/// A set of additional fields to include in the response.
///
/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-extension-members>
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExtensionMembers(serde_json::Map<String, serde_json::Value>);

impl ExtensionMembers
{
	/// Creates a new empty [`ExtensionMembers`].
	pub fn new() -> Self
	{
		Default::default()
	}

	/// Creates a new empty [`ExtensionMembers`] with `capacity` slots pre-allocated.
	pub fn with_capacity(capacity: usize) -> Self
	{
		Self(serde_json::Map::with_capacity(capacity))
	}

	/// Adds an extension member.
	///
	/// Because it is probably a logic error to have duplicates, this function will return the
	/// `value` back to the caller if `key` already exists.
	///
	/// # Panics
	///
	/// This function will panic if `value` cannot be serialized into JSON.
	pub fn add<'this, 'value, K, V>(
		&'this mut self,
		key: K,
		value: &'value V,
	) -> Result<&'this mut serde_json::Value, &'value V>
	where
		K: Into<String>,
		V: Serialize,
	{
		if let serde_json::map::Entry::Vacant(entry) = self.entry(key) {
			let value = serde_json::to_value(value)
				.expect("extension member should be serializable to json");

			Ok(entry.insert(value))
		} else {
			Err(value)
		}
	}
}

impl ops::Deref for ExtensionMembers
{
	type Target = serde_json::Map<String, serde_json::Value>;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}

impl ops::DerefMut for ExtensionMembers
{
	fn deref_mut(&mut self) -> &mut Self::Target
	{
		&mut self.0
	}
}
