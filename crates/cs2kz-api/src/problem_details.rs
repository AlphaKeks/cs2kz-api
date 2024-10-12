//! Our implementation of [`problem_details::ProblemType`].
//!
//! See the [`problem_details`] documentation for more details.

#[repr(u16)]
#[derive(Debug, Clone, Copy)]
pub enum ProblemType {
	Unauthorized = 400,
}

#[derive(Debug, Error)]
#[error("unknown problem")]
pub struct ParseFragmentError;

impl problem_details::ProblemType for ProblemType {
	type ParseFragmentError = ParseFragmentError;

	fn base_uri() -> http::Uri {
		"https://docs.cs2kz.org/api/problems"
			.parse()
			.expect("hard-coded uri should be valid")
	}

	fn parse_fragment(fragment: &str) -> Result<Self, Self::ParseFragmentError> {
		use ProblemType as P;

		match fragment {
			"unauthorized" => Ok(P::Unauthorized),
			_ => Err(ParseFragmentError),
		}
	}

	fn fragment(&self) -> &str {
		use ProblemType as P;

		match self {
			P::Unauthorized => "unauthorized",
		}
	}

	fn status(&self) -> http::StatusCode {
		http::StatusCode::from_u16(*self as u16)
			.expect("hard-coded enum tags should be valid http status codes")
	}

	fn title(&self) -> &str {
		use ProblemType as P;

		match self {
			P::Unauthorized => "unauthorized",
		}
	}
}
