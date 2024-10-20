use std::convert::Infallible;
use std::future::Future;

use axum::response::IntoResponse;
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;
use crate::http::responses::ErrorResponse;
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

	#[instrument(level = "debug", skip_all)]
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

impl AsProblemDetails for InsufficientPermissions {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::Unauthorized
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		if cfg!(not(feature = "production")) {
			_ = extension_members.add("required_permissions", &self.required);
			_ = extension_members.add("actual_permissions", &self.actual);
		}
	}
}

impl AuthorizeSession for RequiredPermissions {
	type Rejection = ErrorResponse;

	#[instrument(level = "debug", skip(_req), err(level = "debug"))]
	async fn authorize_session(
		&mut self,
		_req: &mut http::request::Parts,
		session: &Session,
	) -> Result<(), Self::Rejection> {
		let required = self.0;
		let actual = session.user().permissions();

		if !actual.contains(required) {
			return Err(InsufficientPermissions { required, actual }.into());
		}

		Ok(())
	}
}
