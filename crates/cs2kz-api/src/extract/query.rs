#![allow(clippy::disallowed_types)]

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::http::Problem;

mod base
{
	pub use axum_extra::extract::{Query as Extractor, QueryRejection as Rejection};
}

/// An [extractor] for URI query strings.
///
/// This is the same as [`axum::extract::Query`], except that it produces the same kind of error
/// response as all of our errors.
///
/// [extractor]: axum::extract
/// [request extensions]: http::Request::extensions
#[derive(Debug, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(QueryRejection))]
pub struct Query<T>(pub T);

/// Rejection for the [`Query`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct QueryRejection(#[from] base::Rejection);

impl AsProblemDetails for QueryRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		Problem::InvalidQueryString
	}
}

impl_into_response!(QueryRejection);
