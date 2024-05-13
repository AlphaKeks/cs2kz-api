//! Custom [`Serialize`] / [`Deserialize`] implementations.
//!
//! [`Serialize`]: serde::Serialize
//! [`Deserialize`]: serde::Deserialize

/// (De)serializing empty containers as `None`.
pub mod empty_as_none {
	/// (De)serializing empty [`String`]s as `None`.
	pub mod string {
		use serde::{Deserialize, Deserializer, Serialize, Serializer};

		/// Serializes an [`Option<String>`] as [`None`] if the string is empty.
		pub fn serialize<S>(string: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			string
				.as_ref()
				.filter(|s| !s.is_empty())
				.serialize(serializer)
		}

		/// Deserializes an [`Option<String>`] and converts `Some("")` into `None`.
		pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
		where
			D: Deserializer<'de>,
		{
			Ok(Option::<String>::deserialize(deserializer)?.filter(|s| !s.is_empty()))
		}
	}

	/// (De)serializing empty [`Vec`]s as `None`.
	pub mod vec {
		use serde::{Deserialize, Deserializer, Serialize, Serializer};

		/// Serializes an [`Option<Vec<T>>`] as [`None`] if the vector is empty.
		pub fn serialize<S, T>(vec: &Option<Vec<T>>, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
			T: Serialize,
		{
			vec.as_ref()
				.filter(|vec| !vec.is_empty())
				.serialize(serializer)
		}

		/// Deserializes an [`Option<Vec<T>>`] and converts `Some([])` into `None`.
		pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<Vec<T>>, D::Error>
		where
			D: Deserializer<'de>,
			T: Deserialize<'de>,
		{
			Ok(Option::<Vec<T>>::deserialize(deserializer)?.filter(|vec| !vec.is_empty()))
		}
	}

	/// (De)serializing empty [`BTreeMap`]s as `None`.
	pub mod btree_map {
		use std::collections::BTreeMap;

		use serde::{Deserialize, Deserializer, Serialize, Serializer};

		/// Serializes an [`Option<BTreeMap<K, V>>`] as [`None`] if the vector is empty.
		pub fn serialize<S, K, V>(
			map: &Option<BTreeMap<K, V>>,
			serializer: S,
		) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
			K: Serialize,
			V: Serialize,
		{
			map.as_ref()
				.filter(|map| !map.is_empty())
				.serialize(serializer)
		}

		/// Deserializes an [`Option<BTreeMap<K, V>>`] and converts `Some({})` into `None`.
		pub fn deserialize<'de, D, K, V>(
			deserializer: D,
		) -> Result<Option<BTreeMap<K, V>>, D::Error>
		where
			D: Deserializer<'de>,
			BTreeMap<K, V>: Deserialize<'de>,
		{
			Ok(Option::<BTreeMap<K, V>>::deserialize(deserializer)?.filter(|map| !map.is_empty()))
		}
	}
}

/// (De)serializing containers while enforcing they are not empty.
pub mod non_empty {
	/// (De)serializing [`Vec<T>`]s while enforcing they are not empty.
	pub mod vec {
		use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

		/// Serializes a [`Vec<T>`], but fails if the vector is empty.
		pub fn serialize<S, T>(vec: &Vec<T>, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
			T: Serialize,
		{
			if vec.is_empty() {
				return Err(ser::Error::custom("cannot serialize empty vec"));
			}

			vec.serialize(serializer)
		}

		/// Deserializes into a [`Vec<T>`] and fails if the vector is empty.
		pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
		where
			D: Deserializer<'de>,
			T: Deserialize<'de>,
		{
			let vec = Vec::<T>::deserialize(deserializer)?;

			if vec.is_empty() {
				return Err(de::Error::invalid_length(0, &"more than 0"));
			}

			Ok(vec)
		}
	}
}
