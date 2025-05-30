//! Various [`serde`] helpers

pub mod ser
{
	use {
		serde::{Serialize, Serializer, ser::SerializeSeq},
		std::collections::BTreeMap,
	};

	#[allow(private_bounds)]
	trait Map
	{
		type Value;

		fn len(&self) -> usize;
		fn values(&self) -> impl Iterator<Item = &Self::Value>;
	}

	impl<K, V> Map for BTreeMap<K, V>
	{
		type Value = V;

		fn len(&self) -> usize
		{
			BTreeMap::len(self)
		}

		fn values(&self) -> impl Iterator<Item = &Self::Value>
		{
			BTreeMap::values(self)
		}
	}

	/// Serializes only a map's values as a sequence.
	#[allow(private_bounds)]
	pub fn map_values<T, S>(map: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: Map<Value: Serialize>,
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(Some(map.len()))?;

		for value in map.values() {
			serializer.serialize_element(value)?;
		}

		serializer.end()
	}

	pub mod http
	{
		use {
			http::StatusCode,
			serde::{Serialize, Serializer},
		};

		/// Serializes an [`http::StatusCode`] as an integer.
		pub fn status_code<S>(status_code: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			status_code.as_u16().serialize(serializer)
		}
	}
}

pub mod de
{
	use {
		serde::{Deserialize, Deserializer, de},
		std::collections::{BTreeMap, BTreeSet},
	};

	#[allow(private_bounds)]
	trait IsEmpty
	{
		fn is_empty(&self) -> bool;
	}

	impl<T> IsEmpty for Vec<T>
	{
		fn is_empty(&self) -> bool
		{
			<[T]>::is_empty(self)
		}
	}

	impl<K, V> IsEmpty for BTreeMap<K, V>
	{
		fn is_empty(&self) -> bool
		{
			<BTreeMap<K, V>>::is_empty(self)
		}
	}

	impl<T> IsEmpty for BTreeSet<T>
	{
		fn is_empty(&self) -> bool
		{
			<BTreeSet<T>>::is_empty(self)
		}
	}

	/// Deserializes a collection and ensures it is non-empty.
	#[allow(private_bounds)]
	pub fn non_empty<'de, D, T>(deserializer: D) -> Result<T, D::Error>
	where
		D: Deserializer<'de>,
		T: Deserialize<'de> + IsEmpty,
	{
		let value = T::deserialize(deserializer)?;

		if <T as IsEmpty>::is_empty(&value) {
			return Err(de::Error::custom("must not be empty"));
		}

		Ok(value)
	}
}
