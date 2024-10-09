use axum::response::{IntoResponse, Response};
use problem_details::ProblemDetails;

use crate::database;
use crate::problem_details::ProblemType;

const ERROR: &str = "you are not permitted to perform this action";

#[derive(Debug, Error)]
pub enum SessionRejection {
	#[error("{ERROR}")]
	MissingCookie,

	#[error("{ERROR}")]
	ParseSessionID(#[from] uuid::Error),

	#[error("{ERROR}")]
	InvalidSessionID,

	#[error("something went wrong; please report this incident")]
	Database(#[from] database::Error),
}

impl IntoResponse for SessionRejection {
	fn into_response(self) -> Response {
		#[allow(unused_mut, reason = "we only mutate if debug assertions are enabled")]
		let mut problem_details = ProblemDetails::new(ProblemType::Unauthorized).with_detail(ERROR);

		if cfg!(debug_assertions) {
			let extension_members = problem_details.extension_members_mut();

			match self {
				Self::MissingCookie | Self::InvalidSessionID => {}
				Self::ParseSessionID(source) => {
					_ = extension_members.add("parse_error", &format_args!("{source}"));
				}
				Self::Database(source) => {
					_ = extension_members.add("database_error", &format_args!("{source}"));
				}
			}
		}

		problem_details.into_response()
	}
}
