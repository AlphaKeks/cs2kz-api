//! Our implementation of [`problem_details::ProblemType`].
//!
//! See the [`problem_details`] documentation for more details.

/// The various types of problems that could be referenced by a "problem
/// details" error response.
#[derive(Debug, Clone, Copy)]
#[expect(missing_docs)]
pub enum Problem {
	BadRequest,
	InvalidPathParameters,
	InvalidQueryString,
	MissingHeader,
	InvalidHeaderValue,
	Unauthorized,
	ResourceNotFound,
	ResourceAlreadyExists,
	PluginVersionIsTooOld,
	DeserializeRequestBody,
	Internal,
}

/// Error returned by `<Problem as
/// problem_details::ProblemType>::parse_fragment()`.
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
		match fragment {
			"bad-request" => Ok(Self::BadRequest),
			"invalid-path-parameters" => Ok(Self::InvalidPathParameters),
			"invalid-query-string" => Ok(Self::InvalidQueryString),
			"missing-header" => Ok(Self::MissingHeader),
			"invalid-header-value" => Ok(Self::InvalidHeaderValue),
			"unauthorized" => Ok(Self::Unauthorized),
			"resource-not-found" => Ok(Self::ResourceNotFound),
			"resource-already-exists" => Ok(Self::ResourceAlreadyExists),
			"plugin-version-too-old" => Ok(Self::PluginVersionIsTooOld),
			"deserialize-request-body" => Ok(Self::DeserializeRequestBody),
			"internal" => Ok(Self::Internal),
			_ => Err(ParseFragmentError),
		}
	}

	fn fragment(&self) -> &str {
		match self {
			Self::BadRequest => "bad-request",
			Self::InvalidPathParameters => "invalid-path-parameters",
			Self::InvalidQueryString => "invalid-query-string",
			Self::MissingHeader => "missing-header",
			Self::InvalidHeaderValue => "invalid-header-value",
			Self::Unauthorized => "unauthorized",
			Self::ResourceNotFound => "resource-not-found",
			Self::ResourceAlreadyExists => "resource-already-exists",
			Self::PluginVersionIsTooOld => "plugin-version-too-old",
			Self::DeserializeRequestBody => "deserialize-request-body",
			Self::Internal => "internal",
		}
	}

	fn status(&self) -> http::StatusCode {
		match self {
			Self::BadRequest
			| Self::InvalidPathParameters
			| Self::InvalidQueryString
			| Self::MissingHeader
			| Self::InvalidHeaderValue => http::StatusCode::BAD_REQUEST,
			Self::Unauthorized => http::StatusCode::UNAUTHORIZED,
			Self::ResourceNotFound => http::StatusCode::NOT_FOUND,
			Self::ResourceAlreadyExists | Self::PluginVersionIsTooOld => http::StatusCode::CONFLICT,
			Self::DeserializeRequestBody => http::StatusCode::UNPROCESSABLE_ENTITY,
			Self::Internal => http::StatusCode::INTERNAL_SERVER_ERROR,
		}
	}

	fn title(&self) -> &str {
		match self {
			Self::BadRequest => "bad request",
			Self::InvalidPathParameters => "invalid path parameter(s)",
			Self::InvalidQueryString => "invalid query string",
			Self::MissingHeader => "missing header",
			Self::InvalidHeaderValue => "invalid header value",
			Self::Unauthorized => "unauthorized",
			Self::ResourceNotFound => "resource not found",
			Self::ResourceAlreadyExists => "resource already exists",
			Self::PluginVersionIsTooOld => "plugin version is too old",
			Self::DeserializeRequestBody => "failed to deserialize request body",
			Self::Internal => "internal server error",
		}
	}
}
