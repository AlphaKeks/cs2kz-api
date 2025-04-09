pub use self::{
	version::PluginVersion,
	version_id::{ParsePluginVersionIdError, PluginVersionId},
};
use {
	crate::{
		checksum::Checksum,
		database::{self, DatabaseError, DatabaseResult},
		game::Game,
		git_revision::GitRevision,
		mode::Mode,
		stream::StreamExt as _,
		styles::Style,
		time::Timestamp,
	},
	futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt},
	serde::Serialize,
	sqlx::Row,
	std::ops,
	utoipa::ToSchema,
};

mod version;
mod version_id;

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginVersionInfo
{
	/// A SemVer version.
	pub version: PluginVersion,

	/// The git revision associated with the release commit / tag of this version.
	pub git_revision: GitRevision,

	/// When this version was published.
	pub created_at: Timestamp,
}

#[derive(Debug, Builder)]
pub struct Checksums
{
	pub linux: Checksum,
	pub windows: Checksum,
}

impl ops::Index<Os> for Checksums
{
	type Output = Checksum;

	fn index(&self, os: Os) -> &Self::Output
	{
		match os {
			Os::Linux => &self.linux,
			Os::Windows => &self.windows,
		}
	}
}

impl_rand!(Checksums => |rng| Checksums {
	linux: rng.random(),
	windows: rng.random(),
});

#[derive(Debug, Clone, Copy)]
pub enum Os
{
	Linux,
	Windows,
}

#[derive(Debug, Display, Error, From)]
pub enum CreatePluginVersionError
{
	#[display("version already exists")]
	VersionAlreadyExists,

	#[display("version is older than the latest")]
	VersionOlderThanLatest
	{
		#[error(ignore)]
		latest: PluginVersion,
	},

	#[from(DatabaseError, sqlx::Error)]
	DatabaseError(DatabaseError),
}

#[instrument(skip(db_conn, modes, styles), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create_version<Modes, Styles>(
	#[builder(start_fn)] version: PluginVersion,
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	git_revision: GitRevision,
	linux_checksum: Checksum,
	windows_checksum: Checksum,
	is_cutoff: bool,
	modes: Modes,
	styles: Styles,
) -> Result<(), CreatePluginVersionError>
where
	Modes: IntoIterator<Item = (Mode, Checksums)>,
	Styles: IntoIterator<Item = (Style, Checksums)>,
{
	if let Some(latest) = get_latest_version(game)
		.exec(&mut *db_conn)
		.await?
		.filter(|latest| *latest > version)
	{
		return Err(CreatePluginVersionError::VersionOlderThanLatest { latest });
	}

	let plugin_version_id = sqlx::query!(
		"INSERT INTO PluginVersions (
		   major,
		   minor,
		   patch,
		   pre,
		   build,
		   game,
		   git_revision,
		   linux_checksum,
		   windows_checksum,
		   is_cutoff
		 )
		 VALUES (
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?
		 )
		 RETURNING id",
		version.major(),
		version.minor(),
		version.patch(),
		version.pre(),
		version.build(),
		game,
		git_revision,
		linux_checksum,
		windows_checksum,
		is_cutoff,
	)
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get::<u16, _>(0))
	.map_err(DatabaseError::from)
	.map_err(|err| {
		if err.is_unique_violation("UC_git_revision") && err.is_unique_violation("UC_version") {
			CreatePluginVersionError::VersionAlreadyExists
		} else {
			CreatePluginVersionError::DatabaseError(err)
		}
	})
	.await?;

	for (mode, checksums) in modes {
		sqlx::query!(
			"INSERT INTO ModeChecksums (
			   mode,
			   plugin_version_id,
			   linux_checksum,
			   windows_checksum
			 )
			 VALUES (?, ?, ?, ?)",
			mode,
			plugin_version_id,
			checksums.linux,
			checksums.windows,
		)
		.execute(db_conn.raw_mut())
		.await?;

		debug!(?mode, "inserted mode checksums");
	}

	for (style, checksums) in styles {
		sqlx::query!(
			"INSERT INTO StyleChecksums (
			   style,
			   plugin_version_id,
			   linux_checksum,
			   windows_checksum
			 )
			 VALUES (?, ?, ?, ?)",
			style,
			plugin_version_id,
			checksums.linux,
			checksums.windows,
		)
		.execute(db_conn.raw_mut())
		.await?;

		debug!(?style, "inserted style checksums");
	}

	Ok(())
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count_versions(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	game: Game,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!("SELECT COUNT(*) FROM PluginVersions WHERE game = ?", game)
		.fetch_one(db_conn.raw_mut())
		.map_err(DatabaseError::from)
		.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
		.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_versions(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	game: Game,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<PluginVersionInfo>>
{
	sqlx::query!(
		"SELECT
		   major,
		   minor,
		   patch,
		   pre,
		   build,
		   git_revision AS `git_revision: GitRevision`,
		   created_at AS `created_at: Timestamp`
		 FROM PluginVersions
		 WHERE game = ?
		 ORDER BY id DESC
		 LIMIT ?, ?",
		game,
		offset,
		limit,
	)
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.map_ok(|row| PluginVersionInfo {
		version: PluginVersion::from_parts(row.major, row.minor, row.patch, &row.pre, &row.build),
		git_revision: row.git_revision,
		created_at: row.created_at,
	})
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_latest_version(
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<PluginVersion>>
{
	sqlx::query!(
		"SELECT
		   major,
		   minor,
		   patch,
		   pre,
		   build
		 FROM PluginVersions
		 WHERE game = ?
		 ORDER BY id DESC
		 LIMIT 1",
		game,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_ok(|maybe_row| {
		maybe_row.map(|row| {
			PluginVersion::from_parts(row.major, row.minor, row.patch, &row.pre, &row.build)
		})
	})
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_latest_version_id(
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<PluginVersionId>>
{
	sqlx::query_scalar!(
		"SELECT id AS `id: PluginVersionId`
		 FROM PluginVersions
		 WHERE game = ?
		 ORDER BY id DESC
		 LIMIT 1",
		game,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn validate_checksum(
	#[builder(start_fn)] checksum: &Checksum,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<(PluginVersionId, Game, Os)>>
{
	sqlx::query!(
		"SELECT
		   id AS `id: PluginVersionId`,
		   game AS `game: Game`,
		   (linux_checksum = ?) AS `is_linux!: bool`,
		   (windows_checksum = ?) AS `is_windows!: bool`
		 FROM PluginVersions
		 WHERE (linux_checksum = ? OR windows_checksum = ?)",
		checksum,
		checksum,
		checksum,
		checksum,
	)
	.fetch_optional(db_conn.raw_mut())
	.map_ok(|maybe_row| {
		maybe_row.map(|row| {
			debug_assert!(row.is_linux ^ row.is_windows);
			(row.id, row.game, if row.is_linux { Os::Linux } else { Os::Windows })
		})
	})
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_mode_checksums(
	#[builder(start_fn)] version_id: PluginVersionId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> impl Stream<Item = DatabaseResult<(Mode, Checksums)>>
{
	sqlx::query!(
		"SELECT
		   mode AS `mode: Mode`,
		   linux_checksum AS `linux_checksum: Checksum`,
		   windows_checksum AS `windows_checksum: Checksum`
		 FROM ModeChecksums
		 WHERE plugin_version_id = ?",
		version_id,
	)
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.map_ok(|row| {
		(row.mode, Checksums { linux: row.linux_checksum, windows: row.windows_checksum })
	})
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_style_checksums(
	#[builder(start_fn)] version_id: PluginVersionId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> impl Stream<Item = DatabaseResult<(Style, Checksums)>>
{
	sqlx::query!(
		"SELECT
		   style AS `style: Style`,
		   linux_checksum AS `linux_checksum: Checksum`,
		   windows_checksum AS `windows_checksum: Checksum`
		 FROM StyleChecksums
		 WHERE plugin_version_id = ?",
		version_id,
	)
	.fetch(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.map_ok(|row| {
		(row.style, Checksums { linux: row.linux_checksum, windows: row.windows_checksum })
	})
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete_versions(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<u64>
{
	let cutoff = sqlx::query_scalar!(
		"WITH LatestVersions AS (
		   SELECT *
		   FROM PluginVersions
		   ORDER BY id DESC
		   LIMIT ?
		 )
		 SELECT id
		 FROM LatestVersions
		 ORDER BY id ASC
		 LIMIT 1",
		count,
	)
	.fetch_optional(db_conn.raw_mut())
	.await?;

	sqlx::query!("DELETE FROM PluginVersions WHERE id >= ?", cutoff)
		.execute(db_conn.raw_mut())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}
