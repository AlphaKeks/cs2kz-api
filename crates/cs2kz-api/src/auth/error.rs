use std::convert::Infallible;
use std::fmt;

use problem_details::AsProblemDetails;
use tower_sessions::AuthorizeSession;

use super::SessionStore;
use crate::http::Problem;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error
{
	#[error("invalid session ID")]
	UnknownSessionID,

	#[error("invalid session ID")]
	SessionExpired,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::UnknownSessionID | Self::SessionExpired => Problem::Unauthorized,
			Self::Database(_) => Problem::Internal,
		}
	}
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SessionManagerError<Auth, Inner>(
	#[from] tower_sessions::SessionManagerError<SessionStore, Auth, crate::http::Body, Inner>,
)
where
	Auth: AuthorizeSession<ReqBody = crate::http::Body, Store = SessionStore>,
	Inner: tower::Service<crate::http::Request>,
	Inner::Error: std::error::Error + 'static;

impl<Auth, Inner> AsProblemDetails for SessionManagerError<Auth, Inner>
where
	Auth: AuthorizeSession<ReqBody = crate::http::Body, Store = SessionStore> + fmt::Debug,
	Auth::Error: AsProblemDetails<ProblemType = Problem>,
	Inner: tower::Service<crate::http::Request, Error = Infallible> + fmt::Debug,
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		use tower_sessions::SessionManagerError as E;

		match &self.0 {
			E::MissingSessionID | E::DecodeSessionID(_) => Problem::Unauthorized,
			E::AuthenticateSession(source)
			| E::SaveSession(source)
			| E::InvalidateSession(source) => source.problem_type(),
			E::AuthorizeSession(source) => source.problem_type(),
			E::Service(source) => match *source {},
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers)
	{
		use tower_sessions::SessionManagerError as E;

		match &self.0 {
			E::MissingSessionID | E::DecodeSessionID(_) => {}
			E::AuthenticateSession(source)
			| E::SaveSession(source)
			| E::InvalidateSession(source) => {
				source.add_extension_members(extension_members);
			}
			E::AuthorizeSession(source) => {
				source.add_extension_members(extension_members);
			}
			E::Service(source) => match *source {},
		}
	}
}

#[derive(Debug, Error)]
#[error(transparent)]
pub struct SessionManagerErrorWithoutAuth<Auth, Inner>(
	#[from] tower_sessions::SessionManagerError<SessionStore, Auth, crate::http::Body, Inner>,
)
where
	Auth: AuthorizeSession<ReqBody = crate::http::Body, Store = SessionStore, Error = Infallible>,
	Inner: tower::Service<crate::http::Request>,
	Inner::Error: std::error::Error + 'static;

impl<Auth, Inner> AsProblemDetails for SessionManagerErrorWithoutAuth<Auth, Inner>
where
	Auth: AuthorizeSession<ReqBody = crate::http::Body, Store = SessionStore, Error = Infallible>
		+ fmt::Debug,
	Inner: tower::Service<crate::http::Request, Error = Infallible> + fmt::Debug,
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		use tower_sessions::SessionManagerError as E;

		match &self.0 {
			E::MissingSessionID | E::DecodeSessionID(_) => Problem::Unauthorized,
			E::AuthenticateSession(source)
			| E::SaveSession(source)
			| E::InvalidateSession(source) => source.problem_type(),
			E::AuthorizeSession(source) | E::Service(source) => match *source {},
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers)
	{
		use tower_sessions::SessionManagerError as E;

		match &self.0 {
			E::MissingSessionID | E::DecodeSessionID(_) => {}
			E::AuthenticateSession(source)
			| E::SaveSession(source)
			| E::InvalidateSession(source) => {
				source.add_extension_members(extension_members);
			}
			E::AuthorizeSession(source) | E::Service(source) => match *source {},
		}
	}
}
