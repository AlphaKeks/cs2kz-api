use std::fmt;
use std::ops::{DerefMut, RangeBounds};

use futures::stream::{Stream, TryStreamExt};

use super::PluginVersion;
use crate::database::{self, RowStream};
use crate::git::GitRevision;
use crate::pagination::{Limit, Offset, PaginationResults};
use crate::plugin_versions::{PluginVersionID, PluginVersionName};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	version_id: PluginVersionID,
) -> database::Result<Option<PluginVersion>> {
	let version = sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: GitRevision`,
		   created_at
		 FROM PluginVersions
		 WHERE id = ?
		 ORDER BY created_at DESC",
		version_id,
	)
	.fetch_optional(conn)
	.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_name(
	conn: &mut database::Connection,
	version_name: &PluginVersionName,
) -> database::Result<Option<PluginVersion>> {
	let version = sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: GitRevision`,
		   created_at
		 FROM PluginVersions
		 WHERE name = ?
		 ORDER BY created_at DESC",
		version_name,
	)
	.fetch_optional(conn)
	.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_git_revision(
	conn: &mut database::Connection,
	git_revision: &GitRevision,
) -> database::Result<Option<PluginVersion>> {
	let version = sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: GitRevision`,
		   created_at
		 FROM PluginVersions
		 WHERE git_revision = ?
		 ORDER BY created_at DESC",
		git_revision,
	)
	.fetch_optional(conn)
	.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_latest(
	conn: &mut database::Connection,
) -> database::Result<Option<PluginVersion>> {
	let PaginationResults { mut stream, .. } = self::get(conn, GetPluginVersionsParams {
		limit: Limit::new(1),
		offset: Offset::default(),
	})
	.await?;

	stream.try_next().await
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get(
	conn: &mut database::Connection,
	GetPluginVersionsParams { limit, offset }: GetPluginVersionsParams,
) -> database::Result<PaginationResults<impl RowStream<'_, PluginVersion>>> {
	let total: u64 = sqlx::query_scalar!("SELECT COUNT(id) FROM PluginVersions")
		.fetch_one(&mut *conn)
		.await?
		.try_into()
		.expect("`COUNT()` should return a positive value");

	let stream = sqlx::query_as!(
		PluginVersion,
		"SELECT
		   id `id: PluginVersionID`,
		   name `name: PluginVersionName`,
		   git_revision `git_revision: GitRevision`,
		   created_at
		 FROM PluginVersions
		 ORDER BY created_at DESC
		 LIMIT ?
		 OFFSET ?",
		limit,
		offset,
	)
	.fetch(conn);

	Ok(PaginationResults { total, stream })
}

pub struct GetPluginVersionsParams {
	pub limit: Limit,
	pub offset: Offset,
}
