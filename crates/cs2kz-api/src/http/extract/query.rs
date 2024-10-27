//! The [`Query`] [extractor] and related types.
//!
//! [extractor]: axum::extract

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;

mod base {
	pub use axum_extra::extract::{Query as Extractor, QueryRejection as Rejection};
}

/// An extractor for URI query parameters.
///
/// See [`axum_extra::extract::Query`] for more details.
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(QueryRejection))]
pub struct Query<T>(pub T);

/// Rejection for the [`Query`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct QueryRejection(#[from] base::Rejection);

impl AsProblemDetails for QueryRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::InvalidQueryString
	}
}

impl IntoResponse for QueryRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}
