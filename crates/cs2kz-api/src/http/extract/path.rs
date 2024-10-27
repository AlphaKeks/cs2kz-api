//! The [`Path`] [extractor] and related types.
//!
//! [extractor]: axum::extract

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;

mod base {
	pub use axum::extract::Path as Extractor;
	pub use axum::extract::path::ErrorKind;
	pub use axum::extract::rejection::PathRejection as Rejection;
}

/// An extractor for URI path parameters.
///
/// See [`axum::extract::Path`] for more details.
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(PathRejection))]
pub struct Path<T>(pub T);

/// Rejection for the [`Path`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct PathRejection(#[from] base::Rejection);

impl AsProblemDetails for PathRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		use base::Rejection as R;

		match self.0 {
			R::FailedToDeserializePathParams(_) => Problem::InvalidPathParameters,
			R::MissingPathParams(_) => Problem::Internal,
			_ => Problem::BadRequest,
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		use base::ErrorKind as E;

		let base::Rejection::FailedToDeserializePathParams(ref error) = self.0 else {
			return;
		};

		match error.kind() {
			E::WrongNumberOfParameters { got, expected } => {
				_ = extension_members.add("got", got);
				_ = extension_members.add("expected", expected);
			},
			E::ParseErrorAtKey {
				key,
				value,
				expected_type,
			} => {
				_ = extension_members.add("key", key);
				_ = extension_members.add("value", value);
				_ = extension_members.add("expected_type", expected_type);
			},
			E::ParseErrorAtIndex {
				index,
				value,
				expected_type,
			} => {
				_ = extension_members.add("index", index);
				_ = extension_members.add("value", value);
				_ = extension_members.add("expected_type", expected_type);
			},
			E::ParseError {
				value,
				expected_type,
			} => {
				_ = extension_members.add("value", value);
				_ = extension_members.add("expected_type", expected_type);
			},
			E::InvalidUtf8InPathParam { key } => {
				_ = extension_members.add("key", key);
			},
			E::UnsupportedType { name } => {
				_ = extension_members.add("unsupported_type", name);
			},
			E::Message(message) => {
				_ = extension_members.add("error_message", message);
			},
			_ => {},
		}
	}
}

impl IntoResponse for PathRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}
