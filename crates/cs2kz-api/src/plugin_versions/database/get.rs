use std::fmt;
use std::ops::RangeBounds;

use futures::stream::{Stream, TryStreamExt};

use super::PluginVersion;
use crate::database;
use crate::database::{Limit, Offset};
use crate::git::GitRevision;
use crate::plugin_versions::{PluginVersionID, PluginVersionName};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	version_id: PluginVersionID,
) -> database::Result<Option<PluginVersion>> {
	let version = macros::query!("WHERE id = ?", version_id)
		.fetch_optional(conn)
		.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_name(
	conn: &mut database::Connection,
	version_name: &PluginVersionName,
) -> database::Result<Option<PluginVersion>> {
	let version = macros::query!("WHERE name = ?", version_name)
		.fetch_optional(conn)
		.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_git_revision(
	conn: &mut database::Connection,
	git_revision: &GitRevision,
) -> database::Result<Option<PluginVersion>> {
	let version = macros::query!("WHERE git_revision = ?", git_revision)
		.fetch_optional(conn)
		.await?;

	Ok(version)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_latest(
	conn: &mut database::Connection,
) -> database::Result<Option<PluginVersion>> {
	get(conn, GetPluginVersionsParams {
		limit: 1.into(),
		offset: 0.into(),
	})
	.try_next()
	.await
}

#[instrument(skip(conn))]
pub fn get(
	conn: &mut database::Connection,
	GetPluginVersionsParams { limit, offset }: GetPluginVersionsParams,
) -> impl Stream<Item = database::Result<PluginVersion>> + Unpin + Send + '_ {
	sqlx::query_as! {
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
	}
	.fetch(conn)
}

pub struct GetPluginVersionsParams {
	pub limit: Limit,
	pub offset: Offset,
}

mod macros {
	macro_rules! query {
		($extra_query:literal, $($args:tt)*) => {
			sqlx::query_as! {
				$crate::plugin_versions::database::PluginVersion,
				"SELECT
				   id `id: PluginVersionID`,
				   name `name: PluginVersionName`,
				   git_revision `git_revision: crate::git::GitRevision`,
				   created_at
				 FROM PluginVersions "
				+ $extra_query
				+ " ORDER BY created_at DESC ",
				$($args)*
			}
		};
	}

	pub(super) use query;
}
