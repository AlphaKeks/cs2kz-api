//! This module implements functionality to register new CS2 servers.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::{AccessKey, Host, ServerID, ServerService};
use crate::database::ErrorExt;
use crate::http::Problem;
use crate::util::NonEmpty;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Registers a new server.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn register_server(&self, request: Request) -> Result
	{
		let access_key = AccessKey::new();
		let server_id = sqlx::query! {
			"INSERT INTO Servers
			   (name, host, port, owner_id, access_key)
			 VALUES
			   (?, ?, ?, ?, ?)
			 RETURNING id",
			&*request.name,
			request.host,
			request.port,
			request.owner_id,
			access_key,
		}
		.fetch_one(&self.mysql)
		.await
		.and_then(|row| row.try_get(0))
		.map_err(|error| {
			if error.is_duplicate_of("name") {
				Error::NameAlreadyExists
			} else if error.is_duplicate_of("unique_host_port") {
				Error::HostPortAlreadyExists
			} else {
				Error::Database(error)
			}
		})?;

		Ok(Response {
			server_id,
			access_key,
		})
	}
}

/// Request for registering a new server.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// The server's name.
	pub name: NonEmpty<String>,

	/// The server's connection host (IP / domain).
	pub host: Host,

	/// The server's connection port.
	pub port: u16,

	/// The SteamID of the server owner.
	pub owner_id: SteamID,
}

/// Response for registering a new server.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The server's generated ID.
	pub server_id: ServerID,

	/// Access Key the server can use to authenticate itself with the API.
	pub access_key: AccessKey,
}

/// Errors that can occur when registering a new CS2 server.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("that name is already used")]
	NameAlreadyExists,

	#[error("that host/port combination is already used")]
	HostPortAlreadyExists,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::NameAlreadyExists | Self::HostPortAlreadyExists => Problem::ResourceAlreadyExists,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
