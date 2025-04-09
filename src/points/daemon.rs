//! This module contains the [`PointsDaemon`].
//!
//! It is responsible for watching leaderboards and recalculating points. When
//! leaderboards exceed the [`SMALL_LEADERBOARD_THRESHOLD`] we fit it into a
//! [Normal Inverse Gaussian distribution][norminvgauss], caching the computed
//! parameters in the database. We can then use these paramters to calculate
//! points for newly submitted records. Over time these parameters will become
//! inaccurate, as reality drifts from the previously fitted curve. This is why
//! the points daemon exists: to re-fit the curve and then re-calculate points
//! for all leaderboards.
//!
//! [`SMALL_LEADERBOARD_THRESHOLD`]: super::SMALL_LEADERBOARD_THRESHOLD
//! [norminvgauss]: https://en.wikipedia.org/wiki/Normal-inverse_Gaussian_distribution

use {
	crate::{
		database::{self, DatabaseError, DatabaseResult, QueryBuilder},
		maps::{FilterId, Tier},
		points::{self, DistributionError, Points},
		python::{self, PyState, PythonError},
		records::{self, Leaderboard, LeaderboardEntry},
	},
	futures_util::{TryFutureExt, TryStreamExt},
	pyo3::PyResult,
	std::sync::{Arc, Weak as WeakArc},
	tokio::sync::Notify,
	tokio_util::sync::CancellationToken,
};

/// A background task for re-calculating distribution parameters and
/// leaderboard points.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::points::daemon
#[derive(Debug)]
pub struct PointsDaemon
{
	database: database::ConnectionPool,
	notifications: Arc<Notifications>,
}

#[derive(Debug, Clone)]
pub struct PointsDaemonHandle
{
	notifications: WeakArc<Notifications>,
}

#[derive(Debug, Default)]
struct Notifications
{
	/// A new record has been submitted
	record_submitted: Notify,
}

#[derive(Debug, Display, Error, From)]
pub enum PointsDaemonError
{
	#[display("{_0}")]
	DatabaseError(DatabaseError),

	#[display("{_0}")]
	PythonError(PythonError),

	#[display("failed to calculate distribution: {_0}")]
	CalculateDistribution(DistributionError),

	#[display("failed to calculate points: {message}")]
	CalculatePoints
	{
		#[error(ignore)]
		#[from(ignore)]
		message: Box<str>,
	},
}

impl PointsDaemon
{
	/// Creates a new [`PointsDaemon`].
	pub fn new(database: database::ConnectionPool) -> Self
	{
		Self { database, notifications: Arc::<Notifications>::default() }
	}

	/// Returns a [handle] to this [`PointsDaemon`].
	///
	/// [handle]: PointsDaemonHandle
	pub fn handle(&self) -> PointsDaemonHandle
	{
		PointsDaemonHandle { notifications: Arc::downgrade(&self.notifications) }
	}

	/// Runs the daemon.
	#[instrument(skip(self, cancellation_token), err)]
	pub async fn run(self, cancellation_token: CancellationToken) -> Result<(), PointsDaemonError>
	{
		let Self { database, notifications } = self;
		let mut db_conn;

		while !cancellation_token.is_cancelled() {
			db_conn = database.acquire().await?;

			// step 1: determine the next filter to recalculate
			let Some(filter_to_recalculate) = find_filter_to_recalculate(&mut db_conn).await?
			else {
				// allow other tasks to use this connection while we wait
				drop(db_conn);

				// wait until we either get cancelled or notified that a new
				// record has been submitted
				debug!("waiting for new records to be submitted...");
				select! {
					// we got cancelled -> we're done
					() = cancellation_token.cancelled() => break,

					// we were notified of a new record -> retry `find_filter_to_recalculate`
					() = notifications.record_submitted.notified() => continue,
				};
			};

			// step 2: fetch the filter's leaderboards

			let nub_leaderboard =
				records::get_leaderboard(filter_to_recalculate.id, Leaderboard::NUB)
					.exec(&mut db_conn)
					.try_collect::<Vec<_>>()
					.await?;

			let pro_leaderboard =
				records::get_leaderboard(filter_to_recalculate.id, Leaderboard::PRO)
					.exec(&mut db_conn)
					.try_collect::<Vec<_>>()
					.await?;

			// This is a rare case, but if we got cancelled while fetching the
			// leaderboards, this is our last chance to abort before performing
			// database mutations. If we get past this point, we want to
			// continue until the next iteration to end up in a consistent
			// state.
			if cancellation_token.is_cancelled() {
				break;
			}

			// step 3: re-fit the distributions

			let nub_leaderboard_is_small =
				nub_leaderboard.len() <= points::SMALL_LEADERBOARD_THRESHOLD;

			let pro_leaderboard_is_small =
				pro_leaderboard.len() <= points::SMALL_LEADERBOARD_THRESHOLD;

			let (nub_leaderboard, nub_distribution, pro_leaderboard, pro_distribution) =
				if nub_leaderboard_is_small && pro_leaderboard_is_small {
					(nub_leaderboard, None, pro_leaderboard, None)
				} else {
					// allow other tasks to use this connection while we fit the
					// distribution
					drop(db_conn);

					let (nub_leaderboard, nub_distribution, pro_leaderboard, pro_distribution) =
						python::execute({
							let span = tracing::debug_span!(
								"calculate_distributions",
								filter_id = %filter_to_recalculate.id,
							);

							move |py_state| -> Result<_, DistributionError> {
								let _guard = span.enter();
								let extract_time = |record: &LeaderboardEntry| record.time.as_f64();

								let nub_distribution = if nub_leaderboard_is_small {
									None
								} else {
									tracing::debug_span!(
										"calculate_nub_distribution",
										leaderboard_size = nub_leaderboard.len(),
									)
									.in_scope(|| {
										debug!("calculating nub distribution");
										points::Distribution::fit(
											py_state,
											&nub_leaderboard[..],
											extract_time,
										)
										.inspect(|distribution| debug!(?distribution))
										.map(Some)
									})?
								};

								let pro_distribution = if pro_leaderboard_is_small {
									None
								} else {
									tracing::debug_span!(
										"calculate_pro_distribution",
										leaderboard_size = pro_leaderboard.len(),
									)
									.in_scope(|| {
										debug!("calculating pro distribution");
										points::Distribution::fit(
											py_state,
											&pro_leaderboard[..],
											extract_time,
										)
										.inspect(|distribution| debug!(?distribution))
										.map(Some)
									})?
								};

								Ok((
									nub_leaderboard,
									nub_distribution,
									pro_leaderboard,
									pro_distribution,
								))
							}
						})
						.await??;

					// re-acquire a connection for the rest of the loop
					// iteration
					db_conn = database.acquire().await?;

					(nub_leaderboard, nub_distribution, pro_leaderboard, pro_distribution)
				};

			// step 4: update the cached distribution parameters

			if let Some(ref nub_distribution) = nub_distribution {
				update_distribution_parameter_cache(filter_to_recalculate.id, Leaderboard::NUB)
					.distribution(nub_distribution)
					.exec(&mut db_conn)
					.await?;
			}

			if let Some(ref pro_distribution) = pro_distribution {
				update_distribution_parameter_cache(filter_to_recalculate.id, Leaderboard::PRO)
					.distribution(pro_distribution)
					.exec(&mut db_conn)
					.await?;
			}

			// step 5: recalculate points for the leaderboards

			let (nub_leaderboard, pro_leaderboard) = python::execute({
				let span = tracing::debug_span!(
					"recalculate_points",
					filter_id = %filter_to_recalculate.id,
				);

				move |py_state| -> Result<_, PointsDaemonError> {
					let _guard = span.enter();

					let nub_leaderboard =
						tracing::debug_span!("recalculate_nub_points").in_scope(|| {
							debug!("calculating points for nub leaderboard");
							recalculate_points(nub_leaderboard, Leaderboard::NUB)
								.tier(filter_to_recalculate.nub_tier)
								.maybe_distribution(nub_distribution.as_ref())
								.calculate(py_state)
								.collect::<Result<Vec<_>, _>>()
								.map_err(|err| PointsDaemonError::CalculatePoints {
									message: err.to_string().into_boxed_str(),
								})
						})?;

					let pro_leaderboard =
						tracing::debug_span!("recalculate_pro_points").in_scope(|| {
							debug!("calculating points for pro leaderboard");
							recalculate_points(pro_leaderboard, Leaderboard::PRO)
								.tier(filter_to_recalculate.pro_tier)
								.maybe_distribution(pro_distribution.as_ref())
								.calculate(py_state)
								.collect::<Result<Vec<_>, _>>()
								.map_err(|err| PointsDaemonError::CalculatePoints {
									message: err.to_string().into_boxed_str(),
								})
						})?;

					Ok((nub_leaderboard, pro_leaderboard))
				}
			})
			.await??;

			// step 6: update records with new points

			update_points(filter_to_recalculate.id)
				.nub_leaderboard(nub_leaderboard)
				.pro_leaderboard(pro_leaderboard)
				.exec(&mut db_conn)
				.await?;
		}

		info!("points daemon shutting down");

		Ok(())
	}
}

impl PointsDaemonHandle
{
	/// Notifies the points daemon that a new record has been submitted.
	///
	/// Returns whether the points daemon is still active.
	#[instrument(level = "trace", ret(level = "trace"))]
	pub fn notify_record_submitted(&self) -> bool
	{
		self.notifications
			.upgrade()
			.map(|notifications| notifications.record_submitted.notify_waiters())
			.is_some()
	}
}

#[derive(Debug)]
struct FilterInfo
{
	id: FilterId,
	nub_tier: Tier,
	pro_tier: Tier,
}

/// Returns the ID of the next filter to recalculate.
#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
async fn find_filter_to_recalculate(
	db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<FilterInfo>>
{
	sqlx::query!("LOCK TABLES Filters READ LOCAL, FiltersToRecalculate WRITE")
		.execute(db_conn.raw_mut())
		.await?;

	let maybe_filter = sqlx::query_as!(
		FilterInfo,
		"SELECT
		   id AS `id: FilterId`,
		   nub_tier AS `nub_tier: Tier`,
		   pro_tier AS `pro_tier: Tier`
		 FROM Filters
		 WHERE id = (
		   SELECT filter_id
		   FROM FiltersToRecalculate
		   WHERE priority != 0
		   ORDER BY priority DESC
		   LIMIT 1
		 )",
	)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await?;

	if let Some(FilterInfo { id, .. }) = maybe_filter {
		sqlx::query!(
			"UPDATE FiltersToRecalculate
			 SET priority = 0
			 WHERE filter_id = ?",
			id,
		)
		.execute(db_conn.raw_mut())
		.await?;
	}

	sqlx::query!("UNLOCK TABLES").execute(db_conn.raw_mut()).await?;

	Ok(maybe_filter)
}

/// Updates the `DistributionParameters` table for the given filter.
#[instrument(level = "debug", skip(db_conn), err)]
#[builder(finish_fn = exec)]
async fn update_distribution_parameter_cache(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	distribution: &points::Distribution,
) -> DatabaseResult<()>
{
	sqlx::query(&format! {
		"INSERT INTO {distribution_table} (
		   filter_id,
		   a,
		   b,
		   loc,
		   scale,
		   top_scale
		 ) VALUES (
		   ?,
		   ?,
		   ?,
		   ?,
		   ?,
		   ?
		 )
		 ON DUPLICATE KEY
		 UPDATE a = VALUES(a),
		        b = VALUES(b),
		        loc = VALUES(loc),
		        scale = VALUES(scale),
		        top_scale = VALUES(top_scale)",
		distribution_table = match leaderboard {
			Leaderboard::NUB => "DistributionParameters",
			Leaderboard::PRO => "ProDistributionParameters",
		},
	})
	.bind(filter_id)
	.bind(distribution.a)
	.bind(distribution.b)
	.bind(distribution.loc)
	.bind(distribution.scale)
	.bind(distribution.top_scale)
	.execute(db_conn.raw_mut())
	.await?;

	Ok(())
}

/// Recalculates points for both leaderboards on the given filter and updates
/// the `BestRecords` / `BestProRecords` tables with the results.
#[instrument(level = "debug", skip(py_state), fields(records = records.len()))]
#[builder(finish_fn = calculate)]
fn recalculate_points(
	#[builder(start_fn)] records: Vec<LeaderboardEntry>,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] py_state: &PyState<'_>,
	tier: Tier,
	distribution: Option<&points::Distribution>,
) -> impl Iterator<Item = PyResult<(LeaderboardEntry, Points)>>
{
	debug_assert!(records.is_sorted_by_key(|record| record.time));

	let top_time = records.first().map(|record| record.time);
	let tier_portion = points::TierPortion::new(tier, leaderboard);

	let mut results = distribution.map_or_else(Vec::default, |_| Vec::with_capacity(records.len()));
	let scaled_times = distribution.map_or_else(Vec::default, |distribution| {
		distribution
			.scale(records.iter().map(|record| record.time.as_f64()))
			.collect()
	});

	records.into_iter().enumerate().map(move |(rank, record)| {
		let rank = records::Rank(rank);
		let rank_portion = points::RankPortion::new(rank);
		let leaderboard_portion = if let Some(distribution) = distribution {
			let result = points::LeaderboardPortion::incremental(distribution)
				.scaled_times(&scaled_times[..])
				.results_so_far(&results[..])
				.rank(rank)
				.calculate(py_state)?;

			results.push(result);

			points::LeaderboardPortion((result.as_f64() / distribution.top_scale.as_f64()).min(1.0))
		} else {
			let top_time = top_time.unwrap_or_else(|| {
				unreachable!("`top_time` was taken from `records`, which we currently iterate over")
			});

			points::LeaderboardPortion::for_small_leaderboard(tier, top_time, record.time)
		};

		Ok((record, points::calculate(tier_portion, rank_portion, leaderboard_portion)))
	})
}

/// Updates the `BestRecords` / `BestProRecords` tables.
#[instrument(level = "debug", skip(db_conn, nub_leaderboard, pro_leaderboard), err)]
#[builder(finish_fn = exec)]
async fn update_points(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	nub_leaderboard: impl IntoIterator<Item = (LeaderboardEntry, Points)>,
	pro_leaderboard: impl IntoIterator<Item = (LeaderboardEntry, Points)>,
) -> DatabaseResult<()>
{
	// We don't want to exceed any query length limits.
	const MAX_CHUNK_SIZE: usize = 1000;

	let mut nub_leaderboard = nub_leaderboard.into_iter();
	let mut pro_leaderboard = pro_leaderboard.into_iter();

	db_conn
		.in_transaction(async move |db_conn| {
			let mut nub_query = QueryBuilder::new({
				"INSERT INTO BestRecords (
				   filter_id,
				   player_id,
				   record_id,
				   points
				 )"
			});

			let mut pro_query = QueryBuilder::new({
				"INSERT INTO BestProRecords (
				   filter_id,
				   player_id,
				   record_id,
				   points
				 )"
			});

			let mut has_nub_records = true;
			let mut has_pro_records = true;

			while has_nub_records || has_pro_records {
				if has_nub_records {
					has_nub_records = false;

					nub_query.reset();
					nub_query.push_values(
						nub_leaderboard.by_ref().take(MAX_CHUNK_SIZE),
						|mut query, (record, points)| {
							query.push_bind(filter_id);
							query.push_bind(record.player_id);
							query.push_bind(record.id);
							query.push_bind(points);

							has_nub_records = true;
						},
					);

					if has_nub_records {
						nub_query.push({
							"ON DUPLICATE KEY
							 UPDATE points = VALUES(points)"
						});

						nub_query.build().persistent(false).execute(db_conn.raw_mut()).await?;
					}
				}

				if has_pro_records {
					has_pro_records = false;

					pro_query.reset();
					pro_query.push_values(
						pro_leaderboard.by_ref().take(MAX_CHUNK_SIZE),
						|mut query, (record, points)| {
							query.push_bind(filter_id);
							query.push_bind(record.player_id);
							query.push_bind(record.id);
							query.push_bind(points);

							has_pro_records = true;
						},
					);

					if has_pro_records {
						pro_query.push({
							"ON DUPLICATE KEY
							 UPDATE points = VALUES(points)"
						});

						pro_query.build().persistent(false).execute(db_conn.raw_mut()).await?;
					}
				}
			}

			Ok(())
		})
		.await
}
