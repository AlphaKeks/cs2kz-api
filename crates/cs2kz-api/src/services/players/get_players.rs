//! This module implements functionality to get players.

use cs2kz::SteamID;
use futures::TryStreamExt;
use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};

use super::{get_player, PlayerService};
use crate::http::Problem;
use crate::util::net::IpAddr;
use crate::util::num::ClampedU64;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PlayerService
{
	/// Gets players.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_players(&self, request: Request) -> Result
	{
		let players = sqlx::query! {
			"SELECT
			   u.id `id: SteamID`,
			   u.name,
			   u.ip_address `ip_address: IpAddr`,
			   (
			     SELECT (COUNT(b.id) > 0)
			     FROM Bans b
			     LEFT JOIN Unbans ub ON ub.ban_id = b.id
			     WHERE b.user_id = u.id
			     AND ub.id IS NULL
			   ) is_banned
			 FROM Users u
			 WHERE id > 0
			 AND name LIKE COALESCE(?, u.name)
			 HAVING is_banned = COALESCE(?, is_banned)
			 LIMIT ?
			 OFFSET ?",
			request.name,
			request.is_banned,
			*request.limit,
			*request.offset,
		}
		.fetch(&self.mysql)
		.map_ok(|row| get_player::Response {
			steam_id: row.id,
			name: row.name,
			ip_address: row.ip_address,
			is_banned: row.is_banned.map_or(false, |x| x != 0),
		})
		.try_collect::<Vec<_>>()
		.await?;

		let total = sqlx::query_scalar! {
			"SELECT COUNT(*) FROM (
			   SELECT
			     (
			       SELECT (COUNT(b.id) > 0)
			       FROM Bans b
			       LEFT JOIN Unbans ub ON ub.ban_id = b.id
			       WHERE b.user_id = u.id
			       AND ub.id IS NULL
			     ) is_banned
			   FROM Users u
			   WHERE id > 0
			   AND u.name LIKE COALESCE(?, u.name)
			   HAVING is_banned = COALESCE(?, is_banned)
			  ) bans",
			request.name,
			request.is_banned,
		}
		.fetch_one(&self.mysql)
		.await?
		.try_into()
		.expect("positive count");

		Ok(Response { players, total })
	}
}

/// Request for getting players.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// Only include players whose name matches this query.
	pub name: Option<String>,

	/// Only include players who are (not) banned.
	pub is_banned: Option<bool>,

	/// Limit the amount of players included in the response.
	pub limit: ClampedU64<100, 1000>,

	/// Pagination offset.
	pub offset: ClampedU64,
}

/// Response for getting players.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The players.
	pub players: Vec<get_player::Response>,

	/// How many players matched the query in total (ignoring limits).
	pub total: u64,
}

/// Errors that can occur when getting players.
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
