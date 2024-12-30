//! A background task for continously re-calculating point-distribution data.

use std::collections::{HashMap, hash_map};
use std::iter;
use std::pin::pin;

use cs2kz::{Mode, SteamID, Tier};
use futures::TryStreamExt;
use futures::future::join;
use pyo3::PyErr;
use sqlx::{MySql, Pool, QueryBuilder, Transaction};
use thiserror::Error;
use tokio::select;
use tokio::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

use crate::points::{Distribution, Record, Worker};
use crate::services::maps::FilterID;
use crate::services::records::RecordID;

/// The minimum amount of time to spend each iteration.
const MIN_DELAY: Duration = Duration::from_secs(1);

/// An error returned by the daemon.
#[derive(Debug, Error)]
pub enum DaemonError
{
	/// Python FFI produced an error.
	#[error("python error: {0}")]
	Python(#[from] PyErr),

	/// The database produced an error.
	#[error("database error: {0}")]
	Database(#[from] sqlx::Error),
}

#[allow(clippy::missing_docs_in_private_items)]
struct RawFilter
{
	id: FilterID,
	tier: Tier,
	mode: Mode,
	has_teleports: bool,
}

#[allow(clippy::missing_docs_in_private_items)]
struct Filter
{
	id: FilterID,
	tier: Tier,
}

#[allow(clippy::missing_docs_in_private_items)]
struct Filters
{
	tp: Filter,
	pro: Filter,
}

/// Runs the daemon.
pub async fn run(pool: Pool<MySql>, shutdown_token: CancellationToken) -> Result<(), DaemonError>
{
	let mut worker = Worker::spawn();
	let mut counts = HashMap::new();

	// FIXME: refresh these whenever new maps are approved
	let mut rows = sqlx::query_as!(
		RawFilter,
		"SELECT
		   f.id `id: FilterID`,
		   f.tier `tier: Tier`,
		   f.mode `mode: Mode`,
		   f.teleports `has_teleports: bool`
		 FROM CourseFilters f
		 JOIN Records r ON r.filter_id = f.id
		 JOIN Courses c ON c.id = f.course_id
		 JOIN Maps m ON m.id = c.map_id
		 WHERE m.global_status = 1
		 GROUP BY c.id, f.mode",
	)
	.fetch(&pool);

	let mut filters = Vec::new();

	#[allow(clippy::missing_assert_message)]
	while let Some(vnl_tp) = rows.try_next().await? {
		assert!(vnl_tp.has_teleports);
		assert_eq!(vnl_tp.mode, Mode::Vanilla);

		let vnl_pro = rows.try_next().await?.expect("rows arrive in chunks of 4");
		assert!(!vnl_pro.has_teleports);
		assert_eq!(vnl_pro.mode, Mode::Vanilla);

		let ckz_tp = rows.try_next().await?.expect("rows arrive in chunks of 4");
		assert!(ckz_tp.has_teleports);
		assert_eq!(ckz_tp.mode, Mode::Classic);

		let ckz_pro = rows.try_next().await?.expect("rows arrive in chunks of 4");
		assert!(!ckz_pro.has_teleports);
		assert_eq!(ckz_pro.mode, Mode::Classic);

		filters.push(Filters {
			tp: Filter { id: vnl_tp.id, tier: vnl_tp.tier },
			pro: Filter { id: vnl_pro.id, tier: vnl_pro.tier },
		});

		filters.push(Filters {
			tp: Filter { id: ckz_tp.id, tier: ckz_tp.tier },
			pro: Filter { id: ckz_pro.id, tier: ckz_pro.tier },
		});
	}

	let mut min_delay = pin!(tokio::time::sleep(MIN_DELAY));

	for filters in filters.iter().cycle() {
		select! {
			() = shutdown_token.cancelled() => {
				worker.shutdown().await;
				return Ok(());
			}
			((), Err(error)) = join(min_delay.as_mut(), calculate_for_filter(&pool, &mut worker, &mut counts, filters)) => {
				worker.shutdown().await;
				return Err(error);
			}
			else => {
				min_delay.as_mut().reset(Instant::now() + MIN_DELAY);
				continue;
			},
		}
	}

	unreachable!("the above loop is infinite");
}

/// Calculates point-distribution data for a single filter and updates the
/// database with the results.
async fn calculate_for_filter(
	pool: &Pool<MySql>,
	worker: &mut Worker,
	counts: &mut HashMap<FilterID, u64>,
	filters: &Filters,
) -> Result<(), DaemonError>
{
	/// Checks if the given `filter_id` already exists in `counts`, and if so,
	/// updates the count.
	///
	/// Returns whether anything was actually updated.
	fn check_counts(counts: &mut HashMap<FilterID, u64>, filter_id: FilterID, count: u64) -> bool
	{
		match counts.entry(filter_id) {
			hash_map::Entry::Vacant(entry) => {
				entry.insert(count);
				true
			}
			hash_map::Entry::Occupied(mut entry) => {
				if *entry.get() == count {
					false
				} else {
					entry.insert(count);
					true
				}
			}
		}
	}

	let mut txn = pool.begin().await?;

	let nub_record_count = sqlx::query_scalar!(
		"SELECT COUNT(r.id)
		 FROM Records r
		 JOIN CourseFilters f ON f.id = r.filter_id
		 WHERE f.id = ?",
		filters.tp.id,
	)
	.fetch_one(&mut *txn)
	.await?
	.try_into()
	.expect("`COUNT()` should never return a negative value");

	let pro_record_count = sqlx::query_scalar!(
		"SELECT COUNT(r.id)
		 FROM Records r
		 JOIN CourseFilters f ON f.id = r.filter_id
		 WHERE f.id = ?",
		filters.pro.id,
	)
	.fetch_one(&mut *txn)
	.await?
	.try_into()
	.expect("`COUNT()` should never return a negative value");

	let nub_count_has_changed = check_counts(counts, filters.tp.id, nub_record_count);
	let pro_count_has_changed = check_counts(counts, filters.pro.id, pro_record_count);

	if !nub_count_has_changed && !pro_count_has_changed {
		return Ok(());
	}

	// TODO: filter out cheated runs
	let mut rows = sqlx::query!(
		"SELECT
		   r.id `id: RecordID`,
		   r.player_id `player_id: SteamID`,
		   r.time,
		   f.teleports `has_teleports: bool`
		 FROM CourseFilters f
		 JOIN Records r ON r.filter_id = f.id
		 WHERE f.id IN (?, ?)",
		filters.tp.id,
		filters.pro.id,
	)
	.fetch(&mut *txn);

	let mut nub_records = Vec::with_capacity(nub_record_count as usize);
	let mut pro_records = Vec::with_capacity(pro_record_count as usize);

	while let Some(row) = rows.try_next().await? {
		let record = Record { id: row.id, player_id: row.player_id, time: row.time };

		if row.has_teleports {
			nub_records.push(record);
		} else {
			pro_records.push(record);
		}
	}

	drop(rows);

	if nub_records.is_empty() {
		return Ok(());
	}

	let (nub_records, nub_distribution) = worker.calculate_distribution(nub_records).await?;
	let (pro_records, pro_distribution) = worker.calculate_distribution(pro_records).await?;

	for (filter_id, Distribution { a, b, loc, scale, top_scale }) in
		[(filters.tp.id, &nub_distribution), (filters.pro.id, &pro_distribution)]
	{
		sqlx::query!(
			"INSERT INTO PointDistributionData (filter_id, a, b, loc, scale, top_scale)
			 VALUES (?, ?, ?, ?, ?, ?)
			 ON DUPLICATE KEY
			 UPDATE a = VALUES(a),
				b = VALUES(b),
				loc = VALUES(loc),
				scale = VALUES(scale),
				top_scale = VALUES(top_scale)",
			filter_id,
			a,
			b,
			loc,
			scale,
			top_scale,
		)
		.execute(&mut *txn)
		.await?;
	}

	let (nub_records, nub_points, pro_records, pro_points) = worker
		.calculate_dist_points(
			nub_records,
			nub_distribution,
			filters.tp.tier,
			pro_records,
			pro_distribution,
			filters.pro.tier,
		)
		.await?;

	update_points(&mut txn, filters.tp.id, &nub_records, &nub_points).await?;
	update_points(&mut txn, filters.pro.id, &pro_records, &pro_points).await?;

	txn.commit().await?;

	Ok(())
}

async fn update_points(
	txn: &mut Transaction<'_, MySql>,
	filter_id: FilterID,
	records: &[Record],
	points: &[f64],
) -> Result<(), DaemonError>
{
	/// Limit how many rows we update per query.
	///
	/// This number is fairly arbitrary, we just don't want to exceed any limits
	/// imposed by the database.
	const MAX_CHUNK_SIZE: usize = 10_000;

	// sanity check
	assert_eq!(records.len(), points.len(), "points should correspond to records exactly");

	let mut query = QueryBuilder::new(
		"INSERT INTO BestRecords (
		   player_id,
		   filter_id,
		   record_id,
		   dist_points
		 )",
	);

	for (records, points) in
		iter::zip(records.chunks(MAX_CHUNK_SIZE), points.chunks(MAX_CHUNK_SIZE))
	{
		query.push_values(iter::zip(records, points), |mut query, (record, &points)| {
			query.push_bind(record.player_id);
			query.push_bind(filter_id);
			query.push_bind(record.id);
			query.push_bind(points);
		});

		query.push(
			"ON DUPLICATE KEY
			 UPDATE record_id = VALUES(record_id),
			        dist_points = VALUES(dist_points)",
		);

		query.build().persistent(false).execute(&mut **txn).await?;
		query.reset();
	}

	Ok(())
}
