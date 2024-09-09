//! This module implements functionality to fetch a plugin version by its ID, name, or associated
//! git revision.

use problem_details::AsProblemDetails;
use serde::Serialize;

use super::{PluginService, PluginVersionID, PluginVersionName};
use crate::http::Problem;
use crate::util::time::Timestamp;
use crate::util::GitRevision;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PluginService
{
	/// Gets a specific version by its ID.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_version_by_id(&self, id: PluginVersionID) -> Result
	{
		let version = sqlx::query_as! {
			Response,
			"SELECT
			   id `id: PluginVersionID`,
			   name `name: PluginVersionName`,
			   revision `revision: GitRevision`,
			   created_on `created_on: Timestamp`
			 FROM PluginVersions
			 WHERE id = ?",
			id,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::NotFound)?;

		Ok(version)
	}

	/// Gets a specific version by its name.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_version_by_name(&self, name: &PluginVersionName) -> Result
	{
		let version = sqlx::query_as! {
			Response,
			"SELECT
			   id `id: PluginVersionID`,
			   name `name: PluginVersionName`,
			   revision `revision: GitRevision`,
			   created_on `created_on: Timestamp`
			 FROM PluginVersions
			 WHERE name = ?",
			name,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::NotFound)?;

		Ok(version)
	}

	/// Gets a specific version by its git revision.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_version_by_git_revision(&self, git_revision: &GitRevision) -> Result
	{
		let version = sqlx::query_as! {
			Response,
			"SELECT
			   id `id: PluginVersionID`,
			   name `name: PluginVersionName`,
			   revision `revision: GitRevision`,
			   created_on `created_on: Timestamp`
			 FROM PluginVersions
			 WHERE revision = ?",
			git_revision,
		}
		.fetch_optional(&self.mysql)
		.await?
		.ok_or(Error::NotFound)?;

		Ok(version)
	}
}

/// Response for getting a plugin version.
#[derive(Debug, Serialize)]
pub struct Response
{
	/// The version's ID.
	pub id: PluginVersionID,

	/// The version's SemVer name.
	pub name: PluginVersionName,

	/// The git revision of this version's release/tag.
	pub revision: GitRevision,

	/// When this version was submitted.
	pub created_on: Timestamp,
}

/// Errors that can occur when getting a plugin version.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("version not found")]
	NotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::NotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
