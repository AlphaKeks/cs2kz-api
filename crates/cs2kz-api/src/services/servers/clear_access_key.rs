//! This module implements functionality to clear a CS2 server's access key.

use problem_details::AsProblemDetails;

use super::{ServerID, ServerService};
use crate::http::Problem;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Clears a server's access key.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn clear_access_key(&self, server_id: ServerID) -> Result
	{
		let result = sqlx::query! {
			"UPDATE Servers
			 SET access_key = '00000000-0000-0000-0000-000000000000'
			 WHERE id = ?",
			server_id,
		}
		.execute(&self.mysql)
		.await?;

		match result.rows_affected() {
			0 => return Err(Error::ServerNotFound),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		Ok(())
	}
}

/// Response for clearing a server's access key.
pub type Response = ();

/// Errors that can occur when clearing a server's access key.
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
