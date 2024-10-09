use sqlx::Row;

use crate::database::{self, ErrorExt};
use crate::git::GitRevision;
use crate::plugin_versions::{PluginVersionID, PluginVersionName};

#[instrument(skip(conn), fields(%name, %git_revision), ret(level = "debug"), err(level = "debug"))]
pub async fn create(
	conn: &mut database::Connection,
	name: &PluginVersionName,
	git_revision: &GitRevision,
) -> Result<PluginVersionID, CreatePluginVersionError> {
	let version_id = sqlx::query! {
		"INSERT INTO PluginVersions (name, git_revision)
		 VALUES (?, ?)
		 RETURNING id",
		name,
		git_revision,
	}
	.fetch_one(conn)
	.await
	.and_then(|row| row.try_get(0))
	.map_err(|error| {
		if !error.is_unique_violation() {
			CreatePluginVersionError::Database(error)
		} else if error.message_contains("`name`") {
			CreatePluginVersionError::DuplicateName
		} else if error.message_contains("`git_revision`") {
			CreatePluginVersionError::DuplicateGitRevision
		} else {
			unreachable!();
		}
	})?;

	Ok(version_id)
}

#[derive(Debug, Error)]
pub enum CreatePluginVersionError {
	#[error("duplicate version name")]
	DuplicateName,

	#[error("duplicate git revision")]
	DuplicateGitRevision,

	#[error(transparent)]
	Database(#[from] database::Error),
}
