//! This module implements functionality to fetch plugin versions.

use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};

use super::{get_version, PluginService, PluginVersionID, PluginVersionName};
use crate::http::Problem;
use crate::util::num::ClampedU64;
use crate::util::time::Timestamp;
use crate::util::GitRevision;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PluginService
{
	/// Get many plugin versions.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_versions(&self, request: Request) -> Result
	{
		let versions = sqlx::query_as! {
			get_version::Response,
			"SELECT
			   id `id: PluginVersionID`,
			   name `name: PluginVersionName`,
			   revision `revision: GitRevision`,
			   created_on `created_on: Timestamp`
			 FROM PluginVersions
			 WHERE created_on > COALESCE(?, '1970-01-01 00:00:01')
			 AND created_on < COALESCE(?, '2038-01-19 03:14:07')
			 AND id > COALESCE((SELECT id FROM PluginVersions WHERE name = ?), 0)
			 AND id < COALESCE((SELECT id FROM PluginVersions WHERE name = ?), 1 << 15)
			 LIMIT ?
			 OFFSET ?",
			request.created_after,
			request.created_before,
			request.newer_than,
			request.older_than,
			*request.limit,
			*request.offset,
		}
		.fetch_all(&self.mysql)
		.await?;

		let total = sqlx::query_scalar! {
			"SELECT COUNT(id)
			 FROM PluginVersions
			 WHERE created_on > COALESCE(?, '1970-01-01 00:00:01')
			 AND created_on < COALESCE(?, '2038-01-19 03:14:07')
			 AND id > COALESCE((SELECT id FROM PluginVersions WHERE name = ?), 0)
			 AND id < COALESCE((SELECT id FROM PluginVersions WHERE name = ?), 1 << 15)",
			request.created_after,
			request.created_before,
			request.newer_than,
			request.older_than,
		}
		.fetch_one(&self.mysql)
		.await?
		.try_into()
		.expect("positive count");

		Ok(Response { versions, total })
	}
}

/// Request for getting many plugin versions.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// Only include versions newer than this version.
	pub newer_than: Option<PluginVersionName>,

	/// Only include versions older than this version.
	pub older_than: Option<PluginVersionName>,

	/// Only include versions submitted after this timestamp.
	pub created_after: Option<Timestamp>,

	/// Only include versions submitted before this timestamp.
	pub created_before: Option<Timestamp>,

	/// Limit the maximum number of returned versions.
	#[serde(default)]
	pub limit: ClampedU64<10, 100>,

	/// Pagination offset.
	#[serde(default)]
	pub offset: ClampedU64,
}

/// Response for getting many plugin versions.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The versions.
	pub versions: Vec<get_version::Response>,

	/// The total amount of versions available that match the query, ignoring limits.
	pub total: u64,
}

/// Errors that can occur when getting many plugin versions.
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
