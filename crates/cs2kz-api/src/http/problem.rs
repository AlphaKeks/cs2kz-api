use std::sync::OnceLock;

use problem_details::AsProblemDetails;

static BASE_URI: OnceLock<http::Uri> = OnceLock::new();

pub fn set_base_uri(uri: http::Uri)
{
	drop(BASE_URI.set(uri));
}

problem_type! {
	#[derive(Debug, PartialEq, Error)]
	pub enum Problem
	{
		#[title = "missing header"]
		#[status = BAD_REQUEST]
		MissingHeader = "missing-header",

		#[title = "missing path parameter(s)"]
		#[status = BAD_REQUEST]
		MissingPathParameters = "missing-path-parameters",

		#[title = "invalid header"]
		#[status = BAD_REQUEST]
		InvalidHeader = "invalid-header",

		#[title = "invalid path parameter(s)"]
		#[status = BAD_REQUEST]
		InvalidPathParameters = "invalid-path-parameters",

		#[title = "invalid query string"]
		#[status = BAD_REQUEST]
		InvalidQueryString = "invalid-query-string",

		#[title = "invalid request body"]
		#[status = UNPROCESSABLE_ENTITY]
		InvalidRequestBody = "invalid-request-body",

		#[title = "you are not permitted to perform this action"]
		#[status = UNAUTHORIZED]
		Unauthorized = "unauthorized",

		#[title = "resource not found"]
		#[status = NOT_FOUND]
		ResourceNotFound = "resource-not-found",

		#[title = "submitted resource already exists"]
		#[status = CONFLICT]
		ResourceAlreadyExists = "resource-already-exists",

		#[title = "submitted plugin version is outdated"]
		#[status = CONFLICT]
		OutdatedPluginVersion = "outdated-plugin-version",

		#[title = "provided workshop ID is invalid"]
		#[status = CONFLICT]
		InvalidWorkshopID = "invalid-workshop-id",

		#[title = "unknown mapper"]
		#[status = CONFLICT]
		UnknownMapper = "unknown-mapper",

		#[title = "cannot simultaneously add and remove the same mapper from a map/course"]
		#[status = CONFLICT]
		AddAndRemoveMapper = "add-and-remove-mapper",

		#[title = "maps and courses must always have at least one mapper"]
		#[status = CONFLICT]
		ZeroMappers = "zero-mappers",

		#[title = "supplied course ID does not belong to the map being updated"]
		#[status = CONFLICT]
		InvalidCourseID = "invalid-course-id",

		#[title = "supplied filter ID does not belong to the course being updated"]
		#[status = CONFLICT]
		InvalidFilterID = "invalid-filter-id",

		#[title = "server is currently shutting down; accepting no more requests"]
		#[status = SERVICE_UNAVAILABLE]
		GracefulShutdown = "graceful-shutdown",

		#[title = "internal server error"]
		#[status = INTERNAL_SERVER_ERROR]
		Internal = "internal",

		#[title = "external service returned an error"]
		#[status = BAD_GATEWAY]
		ExternalService = "external-service",

		#[cfg(test)]
		#[title = "whoops"]
		#[status = IM_A_TEAPOT]
		Whoops = "whoops",
	}

	#[derive(Debug, PartialEq, Error)]
	#[error("unknown problem")]
	pub struct UnknownProblem;
}

#[derive(Debug, Error)]
#[error("server is currently shutting down; accepting no more requests")]
pub struct GracefulShutdown;

impl AsProblemDetails for GracefulShutdown
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		Problem::GracefulShutdown
	}
}
