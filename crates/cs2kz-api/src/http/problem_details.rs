//! Our implementation of [`problem_details::ProblemType`].
//!
//! See the [`problem_details`] documentation for more details.

/// The various types of problems that could be referenced by a "problem details" error response.
#[derive(Debug, Clone, Copy)]
pub enum Problem {
	BadRequest,
	MissingPathParameters,
	InvalidPathParameters,
	InvalidQueryString,
	MissingHeader,
	Unauthorized,
	ResourceNotFound,
	DeserializeRequestBody,
	Internal,
}

#[derive(Debug, Error)]
#[error("unknown problem")]
pub struct ParseFragmentError;

impl problem_details::ProblemType for Problem {
	type ParseFragmentError = ParseFragmentError;

	fn base_uri() -> http::Uri {
		"https://docs.cs2kz.org/api/problems"
			.parse()
			.expect("hard-coded uri should be valid")
	}

	fn parse_fragment(fragment: &str) -> Result<Self, Self::ParseFragmentError> {
		use Problem as P;

		match fragment {
			"bad-request" => Ok(P::BadRequest),
			"missing-path-parameters" => Ok(P::MissingPathParameters),
			"invalid-path-parameters" => Ok(P::InvalidPathParameters),
			"invalid-query-string" => Ok(P::InvalidQueryString),
			"missing-header" => Ok(P::MissingHeader),
			"unauthorized" => Ok(P::Unauthorized),
			"resource-not-found" => Ok(P::ResourceNotFound),
			"deserialize-request-body" => Ok(P::DeserializeRequestBody),
			"internal" => Ok(P::Internal),
			_ => Err(ParseFragmentError),
		}
	}

	fn fragment(&self) -> &str {
		use Problem as P;

		match self {
			P::BadRequest => "bad-request",
			P::MissingPathParameters => "missing-path-parameters",
			P::InvalidPathParameters => "invalid-path-parameters",
			P::InvalidQueryString => "invalid-query-string",
			P::MissingHeader => "missing-header",
			P::Unauthorized => "unauthorized",
			P::ResourceNotFound => "resource-not-found",
			P::DeserializeRequestBody => "deserialize-request-body",
			P::Internal => "internal",
		}
	}

	fn status(&self) -> http::StatusCode {
		match self {
			Self::BadRequest
			| Self::MissingPathParameters
			| Self::InvalidPathParameters
			| Self::InvalidQueryString
			| Self::MissingHeader => http::StatusCode::BAD_REQUEST,
			Self::Unauthorized => http::StatusCode::UNAUTHORIZED,
			Self::ResourceNotFound => http::StatusCode::NOT_FOUND,
			Self::DeserializeRequestBody => http::StatusCode::UNPROCESSABLE_ENTITY,
			Self::Internal => http::StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn title(&self) -> &str {
		use Problem as P;

		match self {
			P::BadRequest => "bad request",
			P::MissingPathParameters => "missing path parameter(s)",
			P::InvalidPathParameters => "invalid path parameter(s)",
			P::InvalidQueryString => "invalid query string",
			P::MissingHeader => "missing header",
			P::Unauthorized => "unauthorized",
			P::ResourceNotFound => "resource not found",
			P::DeserializeRequestBody => "failed to deserialize request body",
			P::Internal => "internal server error",
		}
	}
}
