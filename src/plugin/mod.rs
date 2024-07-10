//! Everything related to the [CS2KZ plugin].
//!
//! [CS2KZ plugin]: https://github.com/KZGlobalTeam/cs2kz-metamod

use axum::extract::FromRef;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::make_id::IntoID;
use crate::sqlx::{query, QueryBuilderExt, SqlErrorExt};
use crate::{Error, Result};

mod models;
pub use models::{
	CreatedPluginVersion,
	FetchVersionsRequest,
	NewPluginVersion,
	PluginVersion,
	PluginVersionID,
};

pub mod http;

/// A service for dealing with the KZ plugin as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct PluginService
{
	database: Pool<MySql>,
}

impl PluginService
{
	/// Creates a new [`PluginService`] instance.
	pub const fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}

	/// Fetches plugin versions.
	///
	/// The `limit` and `offset` fields in [`FetchVersionsRequest`] can be used
	/// for pagination. The `u64` part of the returned tuple indicates how many
	/// versions _could_ be fetched; also useful for pagination.
	pub async fn fetch_versions(
		&self,
		request: FetchVersionsRequest,
	) -> Result<(Vec<PluginVersion>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = QueryBuilder::new("SELECT SQL_CALC_FOUND_ROWS * FROM PluginVersions");

		query.push_limits(request.limit, request.offset);

		let plugin_versions = query
			.build_query_as::<PluginVersion>()
			.fetch_all(transaction.as_mut())
			.await?;

		if plugin_versions.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((plugin_versions, total))
	}

	/// Submits a new plugin version.
	pub async fn submit_version(
		&self,
		new_version: NewPluginVersion,
	) -> Result<CreatedPluginVersion>
	{
		let mut transaction = self.database.begin().await?;

		let latest_version = sqlx::query! {
			r#"
			SELECT
			  semver
			FROM
			  PluginVersions
			ORDER BY
			  created_on DESC
			LIMIT
			  1
			"#
		}
		.fetch_optional(transaction.as_mut())
		.await?
		.map(|row| row.semver.parse::<semver::Version>())
		.transpose()
		.map_err(|err| Error::logic("invalid semver in database").context(err))?;

		if let Some(version) = latest_version.filter(|version| version >= &new_version.semver) {
			tracing::warn! {
				target: "cs2kz_api::audit_log",
				latest = %version,
				actual = %new_version.semver,
				"submitted outdated plugin version",
			};

			return Err(Error::outdated_plugin_version(new_version.semver, version));
		}

		let plugin_version_id = sqlx::query! {
			r#"
			INSERT INTO
			  PluginVersions (semver, git_revision)
			VALUES
			  (?, ?)
			"#,
			new_version.semver.to_string(),
			new_version.git_revision,
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_duplicate_entry() {
				Error::already_exists("plugin version").context(err)
			} else {
				Error::from(err)
			}
		})?
		.last_insert_id()
		.into_id::<PluginVersionID>()?;

		transaction.commit().await?;

		tracing::debug! {
			target: "cs2kz_api::audit_log",
			id = %plugin_version_id,
			semver = %new_version.semver,
			git_revision = %new_version.git_revision,
			"created new plugin version",
		};

		Ok(CreatedPluginVersion { plugin_version_id })
	}
}
