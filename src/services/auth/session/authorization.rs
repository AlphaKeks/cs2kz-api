use std::convert::Infallible;
use std::future::Future;

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::{user, Session};
use crate::runtime;

/// An authorization strategy.
///
/// After a session has been extracted, it can be further processed to ensure
/// the user is authorized to perform the action they're trying to perform. This
/// trait allows you to specify this procedure.
///
/// An error returned from [`authorize_session()`] indicates that the
/// authorization failed.
///
/// The default strategy is [`None`], which does nothing, and therefore always
/// succeeds.
///
/// [`authorize_session()`]: AuthorizeSession::authorize_session
pub trait AuthorizeSession: Clone + Sized + Send + Sync + 'static
{
	/// The error type for this authorization strategy.
	type Error: IntoResponse + Send + Sync + 'static;

	/// Authorize the given session.
	fn authorize_session(
		self,
		session: &Session,
		req: &mut Request,
	) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

/// The default authorization strategy.
///
/// Any calls to [`None::authorize_session()`] will always succeed and do
/// nothing.
#[derive(Clone, Copy)]
pub struct None;

impl AuthorizeSession for None
{
	type Error = Infallible;

	async fn authorize_session(
		self,
		_session: &Session,
		_req: &mut Request,
	) -> Result<(), Self::Error>
	{
		Ok(())
	}
}

/// An authorization strategy that checks if the requesting user has certain
/// permissions.
#[derive(Clone, Copy)]
pub struct RequiredPermissions(pub user::Permissions);

/// The error that is returned when a user is lacking the required permissions
/// to perform an action.
#[derive(Debug, Error)]
#[error("you do not have the required permissions to perform this action")]
pub struct InsufficientPermissions
{
	/// The permissions that were requried to perform the action.
	required: user::Permissions,

	/// The permissions that the user actually had.
	actual: user::Permissions,
}

impl IntoResponse for InsufficientPermissions
{
	fn into_response(self) -> Response
	{
		runtime::Error::unauthorized(self).into_response()
	}
}

impl AuthorizeSession for RequiredPermissions
{
	type Error = InsufficientPermissions;

	async fn authorize_session(
		self,
		session: &Session,
		_req: &mut Request,
	) -> Result<(), Self::Error>
	{
		let user_permissions = session.user().permissions();

		if !user_permissions.contains(self.0) {
			return Err(InsufficientPermissions { required: self.0, actual: user_permissions });
		}

		Ok(())
	}
}
