//! This module implements functionality to get a CS2 server's access key.
//!
//! This is for internal use! Normal users should only ever get to see their access key once (when
//! the server is registered or whenever they reset it).

use problem_details::AsProblemDetails;

use super::{AccessKey, ServerID, ServerService};
use crate::http::Problem;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Gets a server's access key.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_access_key(&self, server_id: ServerID) -> Result
	{
		let access_key = sqlx::query_scalar! {
			"SELECT access_key `access_key: AccessKey`
			 FROM Servers
			 WHERE id = ?",
			server_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::ServerNotFound)?;

		Ok(access_key)
	}
}

/// Response for getting a server's access key.
pub type Response = AccessKey;

/// Errors that can occur when getting a server's access key.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("server not found")]
	ServerNotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::ServerNotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
