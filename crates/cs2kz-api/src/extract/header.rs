#![allow(clippy::disallowed_types)]

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::http::Problem;

mod base
{
	pub use axum_extra::typed_header::{
		TypedHeaderRejection as Rejection,
		TypedHeaderRejectionReason as Reason,
	};
	pub use axum_extra::TypedHeader as Extractor;
}

/// An [extractor] for typed headers.
///
/// This is the same as [`axum_extra::TypedHeader`], except that it produces the same kind of error
/// response as all of our errors.
///
/// [extractor]: axum::extract
/// [request extensions]: http::Request::extensions
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(HeaderRejection))]
pub struct Header<T>(pub T);

/// Rejection for the [`Header`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct HeaderRejection(#[from] base::Rejection);

impl AsProblemDetails for HeaderRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self.0.reason() {
			base::Reason::Missing => Problem::MissingHeader,
			base::Reason::Error(_) => Problem::InvalidHeader,
			_ => Problem::InvalidHeader,
		}
	}
}

impl_into_response!(HeaderRejection);
