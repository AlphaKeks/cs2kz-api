use axum::RequestExt;
use cs2kz::SteamID;
use derive_more::{Constructor, Debug};
use problem_details::AsProblemDetails;

use super::{Permissions, Session, SessionStore};
use crate::database;
use crate::extract::Path;
use crate::http::Problem;
use crate::services::servers::{AccessKey, ServerID};

#[derive(Debug, Clone, Copy)]
pub struct HasPermissions(pub Permissions);

#[derive(Debug, Error)]
#[error("insufficient permissions")]
pub struct InsufficientPermissions
{
	required_permissions: Permissions,
}

impl tower_sessions::AuthorizeSession for HasPermissions
{
	type ReqBody = crate::http::Body;
	type Store = SessionStore;
	type Error = InsufficientPermissions;

	async fn authorize_session(
		&mut self,
		session: &Session,
		_: &mut crate::http::Request,
	) -> Result<(), Self::Error>
	{
		if session.data().permissions().contains(self.0) {
			Ok(())
		} else {
			Err(InsufficientPermissions {
				required_permissions: self.0,
			})
		}
	}
}

impl AsProblemDetails for InsufficientPermissions
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		Problem::Unauthorized
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers)
	{
		_ = extension_members.add("required_permissions", &self.required_permissions);
	}
}

impl_into_response!(InsufficientPermissions);

#[derive(Debug, Clone, Constructor)]
pub struct IsServerOwner
{
	#[debug("MySql")]
	mysql: database::Pool,
}

#[derive(Debug, Error)]
pub enum IsServerOwnerRejection
{
	#[error(transparent)]
	PathRejection(#[from] crate::extract::path::PathRejection),

	#[error("unknown server")]
	UnknownServerID,

	#[error("you are not the server owner")]
	Unauthorized,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl tower_sessions::AuthorizeSession for IsServerOwner
{
	type ReqBody = crate::http::Body;
	type Store = SessionStore;
	type Error = IsServerOwnerRejection;

	async fn authorize_session(
		&mut self,
		session: &Session,
		request: &mut crate::http::Request,
	) -> Result<(), Self::Error>
	{
		let Path(server_id) = request.extract_parts::<Path<ServerID>>().await?;
		let server = sqlx::query! {
			"SELECT
			   owner_id `owner_id: SteamID`,
			   access_key `access_key: AccessKey`
			 FROM Servers
			 WHERE id = ?",
			server_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(IsServerOwnerRejection::UnknownServerID)?;

		if server.owner_id != session.data().user_id() {
			return Err(IsServerOwnerRejection::Unauthorized);
		}

		request.extensions_mut().insert(server.access_key);

		Ok(())
	}
}

impl AsProblemDetails for IsServerOwnerRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::PathRejection(source) => source.problem_type(),
			Self::UnknownServerID => Problem::ResourceNotFound,
			Self::Unauthorized => Problem::Unauthorized,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(IsServerOwnerRejection);
