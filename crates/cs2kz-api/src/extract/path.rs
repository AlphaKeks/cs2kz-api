#![allow(clippy::disallowed_types)]

use axum::extract::FromRequestParts;
use problem_details::AsProblemDetails;

use crate::http::Problem;

mod base
{
	pub use axum::extract::path::ErrorKind;
	pub use axum::extract::rejection::PathRejection as Rejection;
	pub use axum::extract::Path as Extractor;
}

/// An [extractor] for [request extensions].
///
/// This is the same as [`axum::extract::Path`], except that it produces the same kind of error
/// response as all of our errors.
///
/// [extractor]: axum::extract
/// [request extensions]: http::Request::extensions
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(PathRejection))]
pub struct Path<T>(pub T);

/// Rejection for the [`Path`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct PathRejection(#[from] base::Rejection);

impl AsProblemDetails for PathRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self.0 {
			base::Rejection::MissingPathParams(_) => Problem::MissingPathParameters,
			base::Rejection::FailedToDeserializePathParams(_) => Problem::InvalidPathParameters,
			_ => Problem::InvalidPathParameters,
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers)
	{
		let base::Rejection::FailedToDeserializePathParams(source) = &self.0 else {
			return;
		};

		match source.kind() {
			base::ErrorKind::WrongNumberOfParameters { got, expected } => {
				extension_members.add("got", &got);
				extension_members.add("expected", &expected);
			}
			base::ErrorKind::ParseErrorAtKey {
				key,
				value,
				expected_type,
			} => {
				extension_members.add("parameter", &key);
				extension_members.add("value", &value);
				extension_members.add("expected_type", expected_type);
			}
			base::ErrorKind::ParseErrorAtIndex {
				index,
				value,
				expected_type,
			} => {
				extension_members.add("idx", &index);
				extension_members.add("value", &value);
				extension_members.add("expected_type", expected_type);
			}
			base::ErrorKind::ParseError {
				value,
				expected_type,
			} => {
				extension_members.add("value", &value);
				extension_members.add("expected_type", expected_type);
			}
			base::ErrorKind::InvalidUtf8InPathParam { key } => {
				extension_members.add("parameter", &key);
			}
			base::ErrorKind::UnsupportedType { name } => {
				extension_members.add("unsupported_type", name);
			}

			_ => {}
		}
	}
}

impl_into_response!(PathRejection);
