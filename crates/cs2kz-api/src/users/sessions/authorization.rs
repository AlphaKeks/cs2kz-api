use std::convert::Infallible;
use std::future::Future;

use axum::response::{IntoResponse, Response};
use problem_details::ProblemDetails;

use crate::problem_details::ProblemType;
use crate::users::permissions::Permissions;
use crate::users::sessions::Session;

/// An authorization strategy.
pub trait AuthorizeSession: Clone + Send + Sync + 'static {
	/// The rejection to return if authorization fails.
	type Rejection: IntoResponse;

	/// Authorizes a session.
	fn authorize_session(
		&mut self,
		req: &mut http::request::Parts,
		session: &Session,
	) -> impl Future<Output = Result<(), Self::Rejection>> + Send;
}

/// An authorization strategy that always succeeds.
///
/// This is useful when the user should be "logged in", but doesn't need to meet any other
/// requirements.
#[derive(Debug, Clone)]
pub struct Noop;

impl AuthorizeSession for Noop {
	type Rejection = Infallible;

	async fn authorize_session(
		&mut self,
		_req: &mut http::request::Parts,
		_session: &Session,
	) -> Result<(), Self::Rejection> {
		Ok(())
	}
}

/// An authorization strategy that checks if the user has certain permissions.
#[derive(Debug, Clone)]
pub struct RequiredPermissions(pub Permissions);

#[derive(Debug, Error)]
#[error("insufficient permissions")]
pub struct InsufficientPermissions {
	required: Permissions,
	actual: Permissions,
}

impl IntoResponse for InsufficientPermissions {
	fn into_response(self) -> Response {
		#[allow(unused_mut, reason = "we only mutate if debug assertions are enabled")]
		let mut problem_details =
			ProblemDetails::new(ProblemType::Unauthorized).with_detail(self.to_string());

		if cfg!(debug_assertions) {
			let extension_members = problem_details.extension_members_mut();
			_ = extension_members.add("required_permissions", &self.required);
			_ = extension_members.add("actual_permissions", &self.actual);
		}

		problem_details.into_response()
	}
}

impl AuthorizeSession for RequiredPermissions {
	type Rejection = InsufficientPermissions;

	async fn authorize_session(
		&mut self,
		_req: &mut http::request::Parts,
		session: &Session,
	) -> Result<(), Self::Rejection> {
		let required = self.0;
		let actual = session.user_permissions;

		if !actual.contains(required) {
			return Err(InsufficientPermissions { required, actual });
		}

		Ok(())
	}
}
