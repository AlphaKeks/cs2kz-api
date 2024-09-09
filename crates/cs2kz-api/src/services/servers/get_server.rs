//! This module implements functionality to get a specific CS2 server by its ID or name.

use cs2kz::SteamID;
use problem_details::AsProblemDetails;
use serde::Serialize;

use super::{Host, ServerID, ServerOwner, ServerService};
use crate::http::Problem;
use crate::util::time::Timestamp;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Gets a specific server by its ID.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_server_by_id(&self, server_id: ServerID) -> Result
	{
		let server = sqlx::query! {
			"SELECT
			   s.id `id: ServerID`,
			   s.name,
			   s.host `host: Host`,
			   s.port,
			   u.id `server_owner_id: SteamID`,
			   u.name server_owner_name,
			   s.created_on `created_on: Timestamp`
			 FROM Servers s
			 JOIN Users u ON u.id = s.owner_id
			 WHERE s.id != 0
			 AND s.id = ?",
			server_id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.map(|row| Response {
			id: row.id,
			name: row.name,
			host: row.host,
			port: row.port,
			owner: ServerOwner {
				steam_id: row.server_owner_id,
				name: row.server_owner_name,
			},
			created_on: row.created_on,
		})
		.ok_or(Error::ServerNotFound)?;

		Ok(server)
	}

	/// Gets a specific server by its name.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_server_by_name(&self, server_name: &str) -> Result
	{
		let server = sqlx::query! {
			"SELECT
			   s.id `id: ServerID`,
			   s.name,
			   s.host `host: Host`,
			   s.port,
			   u.id `server_owner_id: SteamID`,
			   u.name server_owner_name,
			   s.created_on `created_on: Timestamp`
			 FROM Servers s
			 JOIN Users u ON u.id = s.owner_id
			 WHERE s.id != 0
			 AND s.name LIKE ?",
			format!("%{server_name}%"),
		}
		.fetch_optional(&self.mysql)
		.await?
		.map(|row| Response {
			id: row.id,
			name: row.name,
			host: row.host,
			port: row.port,
			owner: ServerOwner {
				steam_id: row.server_owner_id,
				name: row.server_owner_name,
			},
			created_on: row.created_on,
		})
		.ok_or(Error::ServerNotFound)?;

		Ok(server)
	}
}

/// Response for getting a server.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Response
{
	/// The server's ID.
	pub id: ServerID,

	/// The server's name.
	pub name: String,

	/// The server host.
	pub host: Host,

	/// The server port.
	pub port: u16,

	/// The server owner.
	pub owner: ServerOwner,

	/// When the server was approved.
	pub created_on: Timestamp,
}

/// Errors that can occur when getting a server.
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
