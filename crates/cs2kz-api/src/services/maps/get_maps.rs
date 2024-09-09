//! This module implements functionality to get KZ maps.

use std::collections::BTreeMap;

use cs2kz::{MapState, Mode, RankedStatus, SteamID, Tier};
use futures::TryStreamExt;
use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};

use super::{get_map, CourseID, FilterID, MapID, MapService};
use crate::http::Problem;
use crate::services::players::PlayerIdentifier;
use crate::services::steam::{MapFileHash, WorkshopID};
use crate::util::num::ClampedU64;
use crate::util::time::Timestamp;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl MapService
{
	/// Gets maps.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_maps(&self, request: Request) -> Result
	{
		let mut rows = get_map::query! {
			"WHERE m.name LIKE COALESCE(?, m.name)
			 AND m.state = COALESCE(?, m.state)
			 AND m.created_on > COALESCE(?, '1970-01-01 00:00:01')
			 AND m.created_on < COALESCE(?, '2038-01-19 03:14:07')
			 LIMIT ?
			 OFFSET ?",
			request.name,
			request.state,
			request.created_after,
			request.created_before,
			*request.limit,
			*request.offset,
		}
		.fetch(&self.mysql)
		.map_ok(get_map::map_row!());

		let mut maps = BTreeMap::new();

		while let Some(row) = rows.try_next().await? {
			use std::collections::btree_map::Entry;

			match maps.entry(row.id) {
				Entry::Vacant(entry) => {
					entry.insert(row);
				}
				Entry::Occupied(mut entry) => {
					get_map::reduce_result(entry.get_mut(), row);
				}
			}
		}

		if maps.is_empty() {
			return Ok(Response::default());
		}

		let total = sqlx::query_scalar! {
			"SELECT COUNT(id)
			 FROM Maps
			 WHERE name LIKE COALESCE(?, name)
			 AND state = COALESCE(?, state)
			 AND created_on > COALESCE(?, '1970-01-01 00:00:01')
			 AND created_on < COALESCE(?, '2038-01-19 03:14:07')",
			request.name,
			request.state,
			request.created_after,
			request.created_before,
		}
		.fetch_one(&self.mysql)
		.await?
		.try_into()
		.expect("positive count");

		Ok(Response {
			total,
			maps: maps.into_values().collect(),
		})
	}
}

/// Request for getting KZ maps.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// Only include maps whose name matches this query.
	pub name: Option<String>,

	/// Only include maps with this approval state.
	pub state: Option<MapState>,

	/// Only include maps made by this player.
	pub mapper: Option<PlayerIdentifier>,

	/// Only include maps approved after this timestamp.
	pub created_after: Option<Timestamp>,

	/// Only include maps approved before this timestamp.
	pub created_before: Option<Timestamp>,

	/// Limit the amount of maps included in the response.
	pub limit: ClampedU64<1000, 3000>,

	/// Pagination offset.
	pub offset: ClampedU64,
}

/// Response for getting KZ maps.
#[derive(Debug, Default, Serialize)]
pub struct Response
{
	/// The maps.
	pub maps: Vec<get_map::Response>,

	/// How many maps matched the query in total (ignoring limits).
	pub total: u64,
}

/// Errors that can occur when getting KZ maps.
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
