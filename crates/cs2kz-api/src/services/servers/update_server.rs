//! This module implements functionality to update a CS2 server.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;
use serde::Deserialize;

use super::{Host, ServerID, ServerService};
use crate::database::ErrorExt;
use crate::http::Problem;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Updates a CS2 server.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn update_server(&self, server_id: ServerID, request: Request) -> Result
	{
		let result = sqlx::query! {
			"UPDATE Servers
			 SET name = COALESCE(?, name),
			     host = COALESCE(?, host),
			     port = COALESCE(?, port),
			     owner_id = COALESCE(?, owner_id)
			 WHERE id = ?",
			server_id,
			request.new_name,
			request.new_host,
			request.new_port,
			request.new_owner_id,
		}
		.execute(&self.mysql)
		.await
		.map_err(|error| {
			if error.is_fk_violation("Users") {
				Error::NewOwnerNotFound
			} else {
				Error::Database(error)
			}
		})?;

		match result.rows_affected() {
			0 => return Err(Error::ServerNotFound),
			n => assert_eq!(n, 1, "updated more than 1 server"),
		}

		Ok(())
	}
}

/// Request for updating a server.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// A new name.
	pub new_name: Option<String>,

	/// A new host.
	pub new_host: Option<Host>,

	/// A new port.
	pub new_port: Option<u16>,

	/// A new owner.
	pub new_owner_id: Option<SteamID>,
}

/// Response for updating a server.
pub type Response = ();

/// Errors that can occur when updating a server.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("server not found")]
	ServerNotFound,

	#[error("new server owner not found")]
	NewOwnerNotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::ServerNotFound | Self::NewOwnerNotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
