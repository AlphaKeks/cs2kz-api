//! [cs2kz-metamod] integration.
//!
//! [cs2kz-metamod]: https://github.com/KZGlobalTeam/cs2kz-metamod

use std::cmp;

use futures_util::TryStreamExt;
use problem_details::AsProblemDetails;
use sqlx::Row;

use crate::database::{self, DatabaseError, RowStream};
use crate::git;
use crate::http::problem_details::Problem;
use crate::time::Timestamp;

mod plugin_version_id;
pub use plugin_version_id::PluginVersionID;

mod plugin_version_name;
pub use plugin_version_name::PluginVersionName;

/// A `cs2kz-metamod` version.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct PluginVersion {
	/// The version's ID.
	pub id: PluginVersionID,

	/// The version's name.
	///
	/// This is most commonly a [SemVer] identifier, but can also have special
	/// values in local development.
	///
	/// [SemVer]: https://semver.org
	pub name: PluginVersionName,

	/// The git revision associated with this version.
	pub git_revision: git::Revision,

	/// When this version was published.
	pub created_at: Timestamp,
}

/// Returns at most `limit` plugin versions with the specified offset.
///
/// The first element of the returned tuple indicates how many plugin versions
/// are in the database. The second element is a stream that will yield no more
/// than `limit` values.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn get_versions(
	conn: &mut database::Connection,
	limit: u64,
	offset: i64,
) -> database::Result<(u64, impl RowStream<'_, PluginVersion>)> {
	let total: u64 = sqlx::query_scalar!("SELECT COUNT(*) FROM PluginVersions")
		.fetch_one(conn.as_mut())
		.await?
		.try_into()
		.expect("`COUNT()` should return a non-negative value");

	let stream = sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: git::Revision`,
		   created_at `created_at: Timestamp`
		 FROM PluginVersions
		 ORDER BY id DESC
		 LIMIT ?
		 OFFSET ?",
		limit,
		offset,
	)
	.fetch(conn.as_mut())
	.map_err(Into::into);

	Ok((total, stream))
}

/// Returns the plugin version with the specified ID.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn get_version_by_id(
	conn: &mut database::Connection,
	plugin_version_id: PluginVersionID,
) -> database::Result<Option<PluginVersion>> {
	sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: git::Revision`,
		   created_at `created_at: Timestamp`
		 FROM PluginVersions
		 WHERE id = ?",
		plugin_version_id,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Returns the plugin version with the specified git revision.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn get_version_by_git_revision(
	conn: &mut database::Connection,
	git_revision: &git::Revision,
) -> database::Result<Option<PluginVersion>> {
	sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: git::Revision`,
		   created_at `created_at: Timestamp`
		 FROM PluginVersions
		 WHERE git_revision = ?",
		git_revision,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Returns the plugin version with the specified SemVer name.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn get_version_by_semver_ident(
	conn: &mut database::Connection,
	semver: &semver::Version,
) -> database::Result<Option<PluginVersion>> {
	sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: git::Revision`,
		   created_at `created_at: Timestamp`
		 FROM PluginVersions
		 WHERE name = ?",
		semver.to_string(),
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// A new plugin version to be published.
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct NewPluginVersion {
	/// The version's name.
	pub name: PluginVersionName,

	/// The git revision associated with this release.
	pub git_revision: git::Revision,
}

with_database_error! {
	/// Errors that can occur when creating a new plugin version.
	#[derive(Debug, Error)]
	pub enum CreatePluginVersionError {
		/// The version already exists in the database.
		#[error("plugin version has already been published")]
		VersionAlreadyExists,

		/// The version is older than the latest version in the database.
		#[error("plugin version is older than the current latest version ({latest})")]
		VersionTooOld {
			/// The latest version.
			latest: semver::Version,
		},
	}
}

impl AsProblemDetails for CreatePluginVersionError {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		match self {
			Self::VersionAlreadyExists => Problem::ResourceAlreadyExists,
			Self::VersionTooOld { .. } => Problem::PluginVersionIsTooOld,
			Self::Database(error) => error.problem_type(),
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		match self {
			Self::VersionAlreadyExists => {},
			Self::VersionTooOld { latest } => {
				_ = extension_members.add("latest_version", latest);
			},
			Self::Database(error) => error.add_extension_members(extension_members),
		}
	}
}

/// Creates a new plugin version.
///
/// Returns the generated ID.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn create_version(
	conn: &mut database::Connection,
	NewPluginVersion { name, git_revision }: &NewPluginVersion,
) -> Result<PluginVersionID, CreatePluginVersionError> {
	if let Some(version) = get_versions(&mut *conn, 1, 0)
		.await
		.map(|(_, stream)| stream)?
		.try_next()
		.await?
		.map(|version| version.name.into_semver())
	{
		match version.cmp(name.as_semver()) {
			cmp::Ordering::Less => {},
			cmp::Ordering::Equal => {
				return Err(CreatePluginVersionError::VersionAlreadyExists);
			},
			cmp::Ordering::Greater => {
				return Err(CreatePluginVersionError::VersionTooOld { latest: version });
			},
		}
	}

	let plugin_version_id = sqlx::query!(
		"INSERT INTO PluginVersions (name, git_revision)
		 VALUES (?, ?)
		 RETURNING id",
		name,
		git_revision,
	)
	.fetch_one(conn.as_mut())
	.await
	.and_then(|row| row.try_get(0))
	.map_err(DatabaseError::from)
	.map_err(|error| {
		if error.is_unique_violation() {
			CreatePluginVersionError::VersionAlreadyExists
		} else {
			CreatePluginVersionError::Database(error)
		}
	})?;

	Ok(plugin_version_id)
}
