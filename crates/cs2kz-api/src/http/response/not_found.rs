use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;

/// A `404 Not Found` response.
#[derive(Debug, Error)]
#[error("resource not found")]
pub struct NotFound;

impl AsProblemDetails for NotFound {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::ResourceNotFound
	}
}

impl IntoResponse for NotFound {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}
