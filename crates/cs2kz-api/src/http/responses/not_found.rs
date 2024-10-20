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
