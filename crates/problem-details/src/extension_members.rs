//! This module contains the [`ExtensionMembers`] and related types.

use std::ops;

use serde::{Deserialize, Serialize};

/// A set of additional fields to include in the response.
///
/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-extension-members>
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ExtensionMembers(serde_json::Map<String, serde_json::Value>);

impl ExtensionMembers {
	/// Creates a new empty [`ExtensionMembers`].
	pub fn new() -> Self {
		Default::default()
	}

	/// Creates a new empty [`ExtensionMembers`] with `capacity` slots
	/// pre-allocated.
	pub fn with_capacity(capacity: usize) -> Self {
		Self(serde_json::Map::with_capacity(capacity))
	}

	/// Adds an extension member.
	///
	/// If `key` was not already present in the map, a mutable reference to the
	/// newly inserted value is returned inside `Ok`.
	///
	/// Otherwise, a mutable reference to the current value is returned inside
	/// `Err`.
	///
	/// # Panics
	///
	/// This function will panic if `value` cannot be serialized into JSON.
	pub fn add<K, V>(
		&mut self,
		key: K,
		value: &'_ V,
	) -> Result<&mut serde_json::Value, &mut serde_json::Value>
	where
		K: Into<String>,
		V: Serialize + ?Sized,
	{
		match self.entry(key) {
			serde_json::map::Entry::Occupied(entry) => Err(entry.into_mut()),
			serde_json::map::Entry::Vacant(entry) => {
				let serialized = serde_json::to_value(value)
					.expect("extension member should be serializable to json");

				Ok(entry.insert(serialized))
			},
		}
	}
}

impl ops::Deref for ExtensionMembers {
	type Target = serde_json::Map<String, serde_json::Value>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl ops::DerefMut for ExtensionMembers {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}
