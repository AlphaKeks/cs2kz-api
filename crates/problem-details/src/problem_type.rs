//! A trait for generic "problem types".

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A [problem type].
///
/// [problem type]: https://www.rfc-editor.org/rfc/rfc9457.html#name-defining-new-problem-types
pub trait ProblemType: Sized {
	/// Error returned by [`ProblemType::parse_fragment()`].
	type ParseFragmentError: std::error::Error;

	/// The base URI for the URI in the `type` field.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-type>
	fn base_uri() -> http::Uri;

	/// Parses a URI fragment into a problem type.
	///
	/// The fragment produced by [`ProblemType::fragment()`] should always parse
	/// back into the same problem type that it was produced from.
	fn parse_fragment(fragment: &str) -> Result<Self, Self::ParseFragmentError>;

	/// The fragment part of the final `type` URI.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-type>
	fn fragment(&self) -> &str;

	/// The HTTP status code that will be used for the response.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-status>
	fn status(&self) -> http::StatusCode;

	/// A short, human-readable summary of the problem type.
	///
	/// See: <https://www.rfc-editor.org/rfc/rfc9457.html#name-title>
	fn title(&self) -> &str;
}

/// Serializes a [`ProblemType`] into a URI.
///
/// This is used in [`ProblemDetails`]'s [`Serialize`] implementation.
///
/// [`ProblemDetails`]: crate::ProblemDetails
#[derive(Debug)]
pub(crate) struct SerializeProblemType<'a, T>(pub(crate) &'a T)
where
	T: ProblemType;

impl<T> Serialize for SerializeProblemType<'_, T>
where
	T: ProblemType,
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut uri = T::base_uri().to_string();

		if !uri.ends_with('#') {
			uri.push('#');
		}

		uri.push_str(self.0.fragment());
		uri.serialize(serializer)
	}
}

/// Deserializes a [`ProblemType`] from a URI.
///
/// This is used in [`ProblemDetails`]'s [`Serialize`] implementation.
///
/// [`ProblemDetails`]: crate::ProblemDetails
#[derive(Debug)]
pub(crate) struct DeserializeProblemType<T>(pub(crate) T)
where
	T: ProblemType;

impl<'de, T> Deserialize<'de> for DeserializeProblemType<T>
where
	T: ProblemType,
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de::{self, Unexpected};

		let uri = String::deserialize(deserializer)?;
		let (base_uri, fragment) = uri.rsplit_once('#').ok_or_else(|| {
			de::Error::invalid_value(Unexpected::Str(&uri), &"http uri with fragment")
		})?;

		if base_uri != T::base_uri() {
			return Err(de::Error::invalid_value(Unexpected::Str(base_uri), &"correct base uri"));
		}

		T::parse_fragment(fragment).map(Self).map_err(de::Error::custom)
	}
}
