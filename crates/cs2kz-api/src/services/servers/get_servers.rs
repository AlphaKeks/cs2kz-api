//! This module implements functionality to get CS2 servers.

use cs2kz::SteamID;
use futures::TryStreamExt;
use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};

use super::{get_server, Host, ServerID, ServerOwner, ServerService};
use crate::http::Problem;
use crate::services::players::PlayerIdentifier;
use crate::util::num::ClampedU64;
use crate::util::time::Timestamp;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl ServerService
{
	/// Gets servers.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_servers(&self, request: Request) -> Result
	{
		let owner_id = match request.owned_by {
			None => None,
			Some(player) => match player.resolve_id(&self.mysql).await? {
				None => return Ok(Response::default()),
				Some(owner_id) => Some(owner_id),
			},
		};

		let name = request.name.as_deref().map(|name| format!("%{name}%"));

		let servers = sqlx::query! {
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
			 AND s.name LIKE COALESCE(?, s.name)
			 AND s.owner_id = COALESCE(?, s.owner_id)
			 AND s.host = COALESCE(?, s.host)
			 LIMIT ?
			 OFFSET ?",
			name.as_deref(),
			owner_id,
			request.host,
			*request.limit,
			*request.offset,
		}
		.fetch(&self.mysql)
		.map_ok(|row| get_server::Response {
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
		.try_collect::<Vec<_>>()
		.await?;

		let total = sqlx::query_scalar! {
			"SELECT COUNT(id)
			 FROM Servers
			 WHERE id != 0
			 AND name = COALESCE(?, name)
			 AND owner_id = COALESCE(?, owner_id)
			 AND host = COALESCE(?, host)",
			name.as_deref(),
			owner_id,
			request.host,
		}
		.fetch_one(&self.mysql)
		.await?
		.try_into()
		.expect("positive count");

		Ok(Response { servers, total })
	}
}

/// Request for getting servers.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// Only include servers whose name matches this query.
	pub name: Option<String>,

	/// Only include servers owned by this player.
	pub owned_by: Option<PlayerIdentifier>,

	/// Only include servers whose host matches this query.
	pub host: Option<Host>,

	/// Limit the maximum number of returned servers.
	pub limit: ClampedU64<100, 1000>,

	/// Pagination offset.
	pub offset: ClampedU64,
}

/// Response for getting servers.
#[derive(Debug, Default, Serialize)]
pub struct Response
{
	/// The servers.
	pub servers: Vec<get_server::Response>,

	/// The total amount of servers available that match the query, ignoring limits.
	pub total: u64,
}

/// Errors that can occur when getting servers.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
