use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

#[allow(private_bounds)]
pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
	T: IsEmpty + Serialize,
	S: Serializer,
{
	if value.is_empty() {
		return Err(ser::Error::custom(
			"value was not supposed to be empty but is empty",
		));
	}

	value.serialize(serializer)
}

#[allow(private_bounds)]
pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
	T: IsEmpty + Deserialize<'de>,
	D: Deserializer<'de>,
{
	let value = T::deserialize(deserializer)?;

	if value.is_empty() {
		return Err(de::Error::invalid_length(0, &"1 or more"));
	}

	Ok(value)
}

pub mod option
{
	#[allow(clippy::wildcard_imports)]
	use super::*;

	#[allow(private_bounds)]
	pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: IsEmpty + Serialize,
		S: Serializer,
	{
		if value.as_ref().is_some_and(IsEmpty::is_empty) {
			return Err(ser::Error::custom(
				"value was not supposed to be empty but is empty",
			));
		}

		value.serialize(serializer)
	}

	#[allow(private_bounds)]
	pub fn deserialize<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
	where
		T: IsEmpty + Deserialize<'de>,
		D: Deserializer<'de>,
	{
		let value = Option::<T>::deserialize(deserializer)?;

		if value.as_ref().is_some_and(IsEmpty::is_empty) {
			return Err(de::Error::invalid_length(0, &"1 or more"));
		}

		Ok(value)
	}
}

trait IsEmpty
{
	fn is_empty(&self) -> bool;
}

impl<T> IsEmpty for T
where
	T: AsRef<str>,
{
	fn is_empty(&self) -> bool
	{
		<str>::is_empty(self.as_ref())
	}
}
