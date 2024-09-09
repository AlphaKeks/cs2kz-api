//! This module implements functionality to submit new plugin versions.

use std::cmp;

use problem_details::AsProblemDetails;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use super::{PluginService, PluginVersionID, PluginVersionName};
use crate::database::ErrorExt;
use crate::http::Problem;
use crate::util::GitRevision;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PluginService
{
	/// Submits a new plugin version.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn submit_version(&self, request: Request) -> Result
	{
		let latest_version = sqlx::query_scalar! {
			"SELECT name `name: PluginVersionName`
			 FROM PluginVersions
			 ORDER BY created_on DESC
			 LIMIT 1",
		}
		.fetch_optional(&self.mysql)
		.await?;

		match latest_version.map(|v| v.cmp(&request.name)) {
			Some(cmp::Ordering::Equal) => return Err(Error::VersionAlreadyExists),
			Some(cmp::Ordering::Greater) => return Err(Error::VersionTooOld),
			_ => { /* fine */ }
		}

		let version_id = sqlx::query! {
			"INSERT INTO PluginVersions
			   (name, revision)
			 VALUES
			   (?, ?)
			 RETURNING id",
			request.name,
			request.revision,
		}
		.fetch_one(&self.mysql)
		.await
		.and_then(|row| row.try_get(0))
		.map_err(|error| match error.is_duplicate() {
			true => Error::VersionAlreadyExists,
			false => Error::Database(error),
		})?;

		Ok(Response { version_id })
	}
}

/// Request for submitting a new plugin version.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// The SemVer version name.
	pub name: PluginVersionName,

	/// The git revision associated with the release/tag of this version.
	pub revision: GitRevision,
}

/// Response for submitting a new plugin version.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The ID generated for this version.
	pub version_id: PluginVersionID,
}

/// Errors that can occur when submitting a new plugin version.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("submitted version already exists")]
	VersionAlreadyExists,

	#[error("submitted version is older than the current version")]
	VersionTooOld,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::VersionAlreadyExists => Problem::ResourceAlreadyExists,
			Self::VersionTooOld => Problem::OutdatedPluginVersion,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
