//! Trait implementations for the [`serde`] crate.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::Permissions;

impl Serialize for Permissions
{
	fn serialize<S>(&self, serializer: S) -> ::std::result::Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(None)?;

		for value in self {
			serde::ser::SerializeSeq::serialize_element(&mut serializer, value)?;
		}

		serde::ser::SerializeSeq::end(serializer)
	}
}

impl<'de> Deserialize<'de> for Permissions
{
	fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		#[serde(untagged)]
		#[allow(clippy::missing_docs_in_private_items)]
		enum Helper
		{
			Int(u64),
			Word(String),
			Words(Vec<String>),
		}

		Helper::deserialize(deserializer).map(|value| match value {
			Helper::Int(flags) => Self::new(flags),
			Helper::Word(word) => word.parse::<Self>().unwrap_or_default(),
			Helper::Words(words) => words
				.into_iter()
				.flat_map(|word| word.parse::<Self>())
				.fold(Self::NONE, |acc, curr| (acc | curr)),
		})
	}
}
