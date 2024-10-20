use std::fmt;

use axum::response::{IntoResponse, Response};
use problem_details::{AsProblemDetails, ProblemDetails, ProblemType};

use crate::http::problem_details::Problem;

/// A generic HTTP error response.
///
/// This type can be constructed from any error that implements [`AsProblemDetails`].
#[derive(Debug, Error)]
pub struct ErrorResponse(ProblemDetails<Problem>);

impl fmt::Display for ErrorResponse {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0.problem_type().title())?;

		if let Some(detail) = self.0.detail() {
			write!(f, ": {detail}")?;
		}

		Ok(())
	}
}

impl<E> From<E> for ErrorResponse
where
	E: AsProblemDetails<ProblemType = Problem>,
{
	fn from(error: E) -> Self {
		Self(error.as_problem_details())
	}
}

impl IntoResponse for ErrorResponse {
	fn into_response(self) -> Response {
		self.0.into_response()
	}
}
