//! Session authorization.

use std::convert::Infallible;
use std::future::Future;

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;

use crate::database::{self, DatabaseError};
use crate::http::extract::path::{Path, PathRejection};
use crate::http::problem_details::Problem;
use crate::servers::ServerID;
use crate::users::sessions::Session;
use crate::users::{Permissions, UserID};

/// An authorization strategy.
///
/// Types that implement this trait can be used with the [`session_auth`]
/// middleware. They determine whether a session is authorized to make the given
/// request or not. Requests that do not pass the check will be rejected with
/// the [rejection] returned by the authorization strategy.
///
/// [`session_auth`]: crate::http::middleware::session_auth
/// [rejection]: AuthorizeSession::Rejection
pub trait AuthorizeSession: Send + Sync + 'static {
	/// The rejection to return if a request does not pass authorization.
	type Rejection: IntoResponse;

	/// Authorizes the given `session`.
	fn authorize_session(
		&mut self,
		session: &Session,
		request: &mut http::request::Parts,
	) -> impl Future<Output = Result<(), Self::Rejection>> + Send;

	fn or<A>(self, fallback: A) -> Or<Self, A>
	where
		Self: Sized,
		A: AuthorizeSession,
	{
		Or {
			a: self,
			b: fallback,
		}
	}
}

/// The default authorization strategy.
///
/// This is a no-op that always returns `Ok(())`.
#[derive(Debug, Clone)]
pub struct None;

impl AuthorizeSession for None {
	type Rejection = Infallible;

	async fn authorize_session(
		&mut self,
		_: &Session,
		_: &mut http::request::Parts,
	) -> Result<(), Self::Rejection> {
		Ok(())
	}
}

#[derive(Debug, Clone)]
pub struct Or<A, B> {
	a: A,
	b: B,
}

impl<A, B> AuthorizeSession for Or<A, B>
where
	A: AuthorizeSession,
	B: AuthorizeSession,
{
	type Rejection = B::Rejection;

	async fn authorize_session(
		&mut self,
		session: &Session,
		request: &mut http::request::Parts,
	) -> Result<(), Self::Rejection> {
		if self.a.authorize_session(session, request).await.is_ok() {
			Ok(())
		} else {
			self.b.authorize_session(session, request).await
		}
	}
}

/// Requires the user to have certain permissions.
#[derive(Debug, Clone)]
pub struct HasPermissions {
	required: Permissions,
}

impl HasPermissions {
	pub fn new(required: impl Into<Permissions>) -> Self {
		Self {
			required: required.into(),
		}
	}
}

#[derive(Debug, Error)]
#[cfg_attr(
	feature = "production",
	error("you are not permitted to make this request")
)]
#[cfg_attr(not(feature = "production"), error("insufficient permissions"))]
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
		_ = extension_members.add("required_permissions", &self.required);
		_ = extension_members.add("actual_permissions", &self.actual);
	}
}

impl IntoResponse for InsufficientPermissions {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

impl AuthorizeSession for HasPermissions {
	type Rejection = InsufficientPermissions;

	async fn authorize_session(
		&mut self,
		session: &Session,
		_request: &mut http::request::Parts,
	) -> Result<(), Self::Rejection> {
		let required = self.required;
		let actual = session.user().permissions();

		if actual.contains(required) {
			Ok(())
		} else {
			Err(InsufficientPermissions { required, actual })
		}
	}
}

#[derive(Debug, Clone)]
pub struct IsServerOwner {
	database: database::ConnectionPool,
}

impl IsServerOwner {
	pub fn new(database: database::ConnectionPool) -> Self {
		Self { database }
	}
}

#[derive(Debug, Error)]
pub enum IsServerOwnerRejection {
	#[error(transparent)]
	ExtractPathParam(#[from] PathRejection),

	#[cfg_attr(
		feature = "production",
		error("you are not permitted to make this request")
	)]
	#[cfg_attr(not(feature = "production"), error("you are not the server owner"))]
	NotServerOwner,

	#[cfg_attr(
		feature = "production",
		error("something went wrong; please report this incident")
	)]
	#[cfg_attr(not(feature = "production"), error("database error: {0}"))]
	Database(#[from] DatabaseError),
}

impl AsProblemDetails for IsServerOwnerRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		match self {
			Self::ExtractPathParam(rejection) => rejection.problem_type(),
			Self::NotServerOwner => Problem::Unauthorized,
			Self::Database(error) => error.problem_type(),
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		match self {
			Self::ExtractPathParam(rejection) => {
				rejection.add_extension_members(extension_members);
			},
			Self::NotServerOwner => {},
			Self::Database(error) => {
				error.add_extension_members(extension_members);
			},
		}
	}
}

impl IntoResponse for IsServerOwnerRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

impl AuthorizeSession for IsServerOwner {
	type Rejection = IsServerOwnerRejection;

	async fn authorize_session(
		&mut self,
		session: &Session,
		request: &mut http::request::Parts,
	) -> Result<(), Self::Rejection> {
		let Path(server_id) = Path::<ServerID>::from_request_parts(request, &()).await?;
		let mut conn = self.database.get_connection().await?;
		let owner_id = sqlx::query_scalar!(
			"SELECT owner_id `owner_id: UserID`
			 FROM Servers
			 WHERE id = ?",
			server_id,
		)
		.fetch_optional(conn.as_mut())
		.await
		.map_err(DatabaseError::from)?
		.ok_or(IsServerOwnerRejection::NotServerOwner)?;

		if session.user().id() != owner_id {
			return Err(IsServerOwnerRejection::NotServerOwner);
		}

		Ok(())
	}
}
