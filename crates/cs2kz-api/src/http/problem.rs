use std::sync::OnceLock;

use thiserror::Error;

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

		#[title = "invalid header"]
		#[status = BAD_REQUEST]
		InvalidHeader = "invalid-header",

		#[title = "you are not permitted to perform this action"]
		#[status = UNAUTHORIZED]
		Unauthorized = "unauthorized",

		#[title = "something went wrong; please report this incident"]
		#[status = INTERNAL_SERVER_ERROR]
		Internal = "internal",

		#[cfg(test)]
		#[title = "whoops"]
		#[status = IM_A_TEAPOT]
		Whoops = "whoops",
	}

	#[derive(Debug, PartialEq, Error)]
	#[error("unknown problem")]
	pub struct UnknownProblem;
}
