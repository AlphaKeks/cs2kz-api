//! Helper functions for [`serde`].

use serde::{Deserialize, Deserializer};

/// Helper trait for generic functions over `.is_empty()`.
#[sealed]
trait IsEmpty
{
	/// Checks if the container is empty.
	fn is_empty(&self) -> bool;
}

#[sealed]
impl<T> IsEmpty for [T]
{
	fn is_empty(&self) -> bool
	{
		<[T]>::is_empty(self)
	}
}

#[sealed]
impl<T> IsEmpty for Vec<T>
{
	fn is_empty(&self) -> bool
	{
		<[T]>::is_empty(&self[..])
	}
}

#[sealed]
impl IsEmpty for str
{
	fn is_empty(&self) -> bool
	{
		<str>::is_empty(self)
	}
}

#[sealed]
impl IsEmpty for String
{
	fn is_empty(&self) -> bool
	{
		<str>::is_empty(self.as_str())
	}
}

/// Deserializes a `Vec<T>` and makes sure it isn't empty.
pub fn deserialize_non_empty<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
	T: IsEmpty + Deserialize<'de>,
	D: Deserializer<'de>,
{
	let v = T::deserialize(deserializer)?;

	if v.is_empty() {
		return Err(serde::de::Error::invalid_length(0, &"1 or more"));
	}

	Ok(v)
}

/// Deserializes an `Option<T>` but treats `Some(<empty>)` as `None`.
pub fn deserialize_empty_as_none<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
	T: IsEmpty,
	Option<T>: Deserialize<'de>,
	D: Deserializer<'de>,
{
	Ok(Option::<T>::deserialize(deserializer)?.filter(|v| !v.is_empty()))
}
