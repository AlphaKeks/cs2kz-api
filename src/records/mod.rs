pub use self::{
	id::{ParseRecordIdError, RecordId},
	points::{InvalidPoints, Points},
	rank::Rank,
	teleports::Teleports,
	time::{InvalidTime, Time},
};
use {
	crate::{
		database::{self, DatabaseError, DatabaseResult, QueryBuilder},
		maps::{CourseId, CourseLocalId, CourseName, FilterId, MapId, MapName, Tier},
		mode::Mode,
		players::{self, PlayerId, PlayerName, PlayerRating},
		points::CalculateLeaderboardPortionForNewRecordError,
		servers::ServerSessionId,
		stream::StreamExt as _,
		styles::Styles,
		time::Timestamp,
	},
	futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt},
	serde::{Deserialize, Serialize},
	sqlx::Row,
	utoipa::ToSchema,
};

mod id;
mod points;
mod rank;
mod teleports;
mod time;

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct Record
{
	pub id: RecordId,

	#[sqlx(flatten)]
	pub player: PlayerInfo,

	#[sqlx(flatten)]
	pub map: MapInfo,

	#[sqlx(flatten)]
	pub course: CourseInfo,

	pub mode: Mode,
	pub styles: Styles,
	pub time: Time,
	pub teleports: Teleports,
	pub nub_points: Option<Points>,
	pub pro_points: Option<Points>,
	pub created_at: Timestamp,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct PlayerInfo
{
	#[sqlx(rename = "player_id")]
	pub id: PlayerId,

	#[sqlx(rename = "player_name")]
	pub name: PlayerName,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct MapInfo
{
	#[sqlx(rename = "map_id")]
	pub id: MapId,

	#[sqlx(rename = "map_name")]
	pub name: MapName,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct CourseInfo
{
	#[sqlx(rename = "course_id")]
	pub id: CourseId,

	#[sqlx(rename = "course_local_id")]
	pub local_id: CourseLocalId,

	#[sqlx(rename = "course_name")]
	pub name: CourseName,
}

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct LeaderboardEntry
{
	pub id: RecordId,
	pub player_id: PlayerId,
	pub time: Time,
}

#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Leaderboard
{
	NUB,
	PRO,
}

#[derive(Debug)]
pub struct CreatedRecord
{
	pub id: RecordId,
	pub ranked_data: Option<CreatedRankedRecordData>,
}

#[derive(Debug, Serialize)]
pub struct CreatedRankedRecordData
{
	pub nub_stats: Option<CreatedRankedRecordStats>,
	pub pro_stats: Option<CreatedRankedRecordStats>,
	pub player_rating: PlayerRating,
}

#[derive(Debug, Serialize)]
pub struct CreatedRankedRecordStats
{
	pub leaderboard_size: usize,
	pub rank: Rank,
	pub points: Points,

	/// Players whose ratings have been affected by point recalculations caused
	/// by this new record
	///
	/// This will usually be empty. If `rank <= SMALL_LEADERBOARD_THRESHOLD`
	/// however, all records slower than this record whose
	/// `rank <= SMALL_LEADERBOARD_THRESHOLD` will have to be recalculated.
	pub players_to_recalc: Vec<PlayerId>,
}

#[derive(Debug, Display, Error, From)]
#[display("failed to create record: {_variant}")]
pub enum CreateRecordError
{
	#[display("player is banned")]
	PlayerIsBanned,

	#[from]
	CalculatePoints(CalculateLeaderboardPortionForNewRecordError),

	#[from(DatabaseError, sqlx::Error)]
	DatabaseError(DatabaseError),
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	session_id: ServerSessionId,
	time: Time,
	teleports: Teleports,
	styles: Styles,
) -> Result<CreatedRecord, CreateRecordError>
{
	#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
	#[builder(finish_fn = exec)]
	async fn update_leaderboard(
		#[builder(start_fn)] filter_id: FilterId,
		#[builder(start_fn)] leaderboard: Leaderboard,
		#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
		time: Time,
		has_pb: bool,
	) -> Result<CreatedRankedRecordStats, CreateRecordError>
	{
		let tier = {
			let mut query = QueryBuilder::new("SELECT");

			query.push(match leaderboard {
				Leaderboard::NUB => " nub_tier ",
				Leaderboard::PRO => " pro_tier ",
			});

			query.push("FROM Filters WHERE id = ?");

			query
				.build_query_scalar::<Tier>()
				.bind(filter_id)
				.fetch_one(db_conn.raw_mut())
				.await?
		};

		let leaderboard_entries = {
			#[derive(Debug, sqlx::FromRow)]
			struct LeaderboardEntry
			{
				player_id: PlayerId,
				time: Time,
			}

			let mut query = QueryBuilder::new("SELECT r.player_id, r.time FROM");

			query.push(match leaderboard {
				Leaderboard::NUB => " BestRecords ",
				Leaderboard::PRO => " BestProRecords ",
			});

			query.push({
				"AS br
				 INNER JOIN Records AS r ON r.id = br.record_id
				 WHERE br.filter_id = ?
				 ORDER BY r.time ASC, r.created_at ASC"
			});

			query
				.build_query_as::<LeaderboardEntry>()
				.bind(filter_id)
				.fetch_all(db_conn.raw_mut())
				.await?
		};

		// Predict our rank by doing a binary search.
		let rank = {
			let mut to_search = &leaderboard_entries[..];
			let mut skipped = 0;

			Rank(loop {
				// If `binary_search()` returns `Ok`, an existing record has the
				// exact same time as this new record. If there are multiple
				// such records, it is not guaranteed which of them will be
				// returned. Because older records win a tie on `time`, we need
				// to find the right-most index that does *not* match our time
				// (returns `Err`).
				match to_search.binary_search_by_key(&time, |entry| entry.time) {
					// we found an identical time -> binary search *again* but
					// strictly *after* this index
					Ok(idx) => {
						to_search = &to_search[(idx + 1)..];
						skipped += (idx + 1);
					},

					// we found the index at which we *would* be if the
					// leaderboard contained us -> return that
					Err(idx) => {
						break (idx + skipped);
					},
				}
			})
		};

		let points = 'points: {
			let is_small_leaderboard =
				leaderboard_entries.len() <= crate::points::SMALL_LEADERBOARD_THRESHOLD;

			let leaderboard_portion = if !is_small_leaderboard
				&& let Some(distribution) =
					crate::points::Distribution::get_cached(filter_id, leaderboard)
						.exec(&mut *db_conn)
						.await?
			{
				crate::points::LeaderboardPortion::from_distribution(distribution, time).await?
			} else if let Some(top_time) = leaderboard_entries
				.first()
				.map(|entry| entry.time)
				.filter(|&top_time| top_time < time)
			{
				crate::points::LeaderboardPortion::for_small_leaderboard(tier, top_time, time)
			} else {
				break 'points Points::MAX;
			};

			crate::points::calculate(
				crate::points::TierPortion::new(tier, leaderboard),
				crate::points::RankPortion::new(rank),
				leaderboard_portion,
			)
		};

		let leaderboard_size = leaderboard_entries.len() + usize::from(!has_pb);
		let players_to_recalc = leaderboard_entries
			.into_iter()
			.enumerate()
			.skip(rank.0 - 1)
			.take_while(|&(rank, _)| rank < crate::points::SMALL_LEADERBOARD_THRESHOLD)
			.map(|(_, entry)| entry.player_id)
			.collect::<Vec<_>>();

		Ok(CreatedRankedRecordStats {
			// If the player didn't have a previous PB, this run increases the
			// leaderboard size by 1.
			leaderboard_size,
			rank,
			points,
			players_to_recalc,
		})
	}

	#[instrument(level = "debug", skip(db_conn), ret(level = "debug"), err)]
	#[builder(finish_fn = exec)]
	async fn update_pb(
		#[builder(start_fn)] filter_id: FilterId,
		#[builder(start_fn)] player_id: PlayerId,
		#[builder(start_fn)] record_id: RecordId,
		#[builder(start_fn)] leaderboard: Leaderboard,
		#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
		points: Points,
	) -> DatabaseResult<()>
	{
		let mut query = QueryBuilder::new("INSERT INTO");

		query.push(match leaderboard {
			Leaderboard::NUB => " BestRecords ",
			Leaderboard::PRO => " BestProRecords ",
		});

		query.push({
			"(filter_id, player_id, record_id, points)
			 VALUES (?, ?, ?, ?)
			 ON DUPLICATE KEY
			 UPDATE record_id = VALUES(record_id),
			        points = VALUES(points)"
		});

		query
			.build()
			.bind(filter_id)
			.bind(player_id)
			.bind(record_id)
			.bind(points)
			.execute(db_conn.raw_mut())
			.await?;

		Ok(())
	}

	if players::is_banned(player_id).exec(&mut *db_conn).await? {
		return Err(CreateRecordError::PlayerIsBanned);
	}

	let record_id = sqlx::query!(
		"INSERT INTO Records (filter_id, player_id, session_id, time, teleports, styles)
		 VALUES (?, ?, ?, ?, ?, ?)
		 RETURNING id",
		filter_id,
		player_id,
		session_id,
		time,
		teleports,
		styles,
	)
	.fetch_one(db_conn.raw_mut())
	.and_then(async |row| row.try_get(0))
	.await?;

	if !styles.is_empty() {
		return Ok(CreatedRecord { id: record_id, ranked_data: None });
	}

	let nub_pb = sqlx::query!(
		"SELECT r.time AS `time: Time`
		 FROM BestRecords AS br
		 INNER JOIN Records as r ON r.id = br.record_id
		 WHERE br.filter_id = ?
		 AND br.player_id = ?",
		filter_id,
		player_id,
	)
	.fetch_optional(db_conn.raw_mut())
	.await?;

	let pro_pb = sqlx::query!(
		"SELECT r.time AS `time: Time`
		 FROM BestProRecords AS br
		 INNER JOIN Records as r ON r.id = br.record_id
		 WHERE br.filter_id = ?
		 AND br.player_id = ?",
		filter_id,
		player_id,
	)
	.fetch_optional(db_conn.raw_mut())
	.await?;

	let nub_stats = if nub_pb.as_ref().is_none_or(|pb| pb.time > time) {
		let stats = update_leaderboard(filter_id, Leaderboard::NUB)
			.time(time)
			.has_pb(nub_pb.is_some())
			.exec(&mut *db_conn)
			.await?;

		update_pb(filter_id, player_id, record_id, Leaderboard::NUB)
			.points(stats.points)
			.exec(&mut *db_conn)
			.await?;

		Some(stats)
	} else {
		None
	};

	let pro_stats = if teleports.as_u32() == 0 && pro_pb.as_ref().is_none_or(|pb| pb.time > time) {
		let stats = update_leaderboard(filter_id, Leaderboard::PRO)
			.time(time)
			.has_pb(nub_pb.is_some())
			.exec(&mut *db_conn)
			.await?;

		update_pb(filter_id, player_id, record_id, Leaderboard::PRO)
			.points(stats.points)
			.exec(&mut *db_conn)
			.await?;

		Some(stats)
	} else {
		None
	};

	let player_rating = if nub_stats.is_some() || pro_stats.is_some() {
		players::recalculate_rating(player_id).exec(&mut *db_conn).await?
	} else {
		None
	};

	Ok(CreatedRecord {
		id: record_id,
		ranked_data: player_rating.map(|player_rating| CreatedRankedRecordData {
			nub_stats,
			pro_stats,
			player_rating,
		}),
	})
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	player: Option<PlayerId>,
	course: Option<CourseId>,
	mode: Option<Mode>,
	top: bool,
	pro: bool,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar::<_, i64>(&format! {
		"SELECT COUNT(DISTINCT r.id)
		 FROM Records AS r
		 {}
		 INNER JOIN Filters AS f ON f.id = r.filter_id
		 WHERE {}
		 AND r.player_id = COALESCE(?, r.player_id)
		 AND f.course_id = COALESCE(?, f.course_id)
		 AND f.mode = COALESCE(?, f.mode)",
		match (top, pro) {
			(true, true) => "INNER JOIN BestProRecords AS br ON br.record_id = r.id",
			(true, false) => "INNER JOIN BestRecords AS br ON br.record_id = r.id",
			_ => "",
		},
		match (top, pro) {
			(false, true) => "r.teleports = 0",
			_ => "TRUE",
		},
	})
	.bind(player)
	.bind(course)
	.bind(mode)
	.fetch_one(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.and_then(async |count| count.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	player: Option<PlayerId>,
	course: Option<CourseId>,
	mode: Option<Mode>,
	top: bool,
	pro: bool,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Record>>
{
	let (conn, query) = db_conn.parts();
	query.reset();

	query.push(format_args! {
		"SELECT
		   r.id,
		   p.id AS player_id,
		   p.name AS player_name,
		   m.id AS map_id,
		   m.name AS map_name,
		   c.id AS course_id,
		   c.local_id AS course_local_id,
		   c.name AS course_name,
		   f.mode,
		   r.styles,
		   r.time,
		   r.teleports,
		   BestRecords.points AS nub_points,
		   BestProRecords.points AS pro_points,
		   r.created_at
		 FROM Records AS r
		 {best_nub_records_join_kind} JOIN BestRecords ON BestRecords.record_id = r.id
		 {best_pro_records_join_kind} JOIN BestProRecords ON BestProRecords.record_id = r.id
		 INNER JOIN Players AS p ON p.id = r.player_id
		 INNER JOIN Filters AS f ON f.id = r.filter_id
		 INNER JOIN Courses AS c ON c.id = f.course_id
		 INNER JOIN Maps AS m ON m.id = c.map_id
		 WHERE {0}
		 AND r.player_id = COALESCE(?, r.player_id)
		 AND c.id = COALESCE(?, c.id)
		 AND f.mode = COALESCE(?, f.mode)
		 ORDER BY {order_by}
		 LIMIT ?, ?",
		if pro { "r.teleports = 0" } else { "TRUE" },
		best_nub_records_join_kind = if top { "INNER" } else { "LEFT" },
		best_pro_records_join_kind = if top && pro { "INNER" } else { "LEFT" },
		order_by = match (top, pro) {
			(true, true) => "BestProRecords.points DESC, r.id DESC",
			(true, false) => "BestRecords.points DESC, r.id DESC",
			(false, _) => "r.id ASC",
		},
	});

	query
		.build_query_as()
		.bind(player)
		.bind(course)
		.bind(mode)
		.bind(dbg!(offset))
		.bind(dbg!(limit))
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] record_id: RecordId,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<Option<Record>>
{
	sqlx::query_as::<_, Record>({
		"SELECT
		   r.id,
		   p.id AS player_id,
		   p.name AS player_name,
		   m.id AS map_id,
		   m.name AS map_name,
		   c.id AS course_id,
		   c.local_id AS course_local_id,
		   c.name AS course_name,
		   f.mode,
		   r.styles,
		   r.time,
		   r.teleports,
		   BestRecords.points AS nub_points,
		   BestProRecords.points AS pro_points,
		   r.created_at
		 FROM Records AS r
		 LEFT JOIN BestRecords ON BestRecords.record_id = r.id
		 LEFT JOIN BestProRecords ON BestProRecords.record_id = r.id
		 INNER JOIN Players AS p ON p.id = r.player_id
		 INNER JOIN Filters AS f ON f.id = r.filter_id
		 INNER JOIN Courses AS c ON c.id = f.course_id
		 INNER JOIN Maps AS m ON m.id = c.map_id
		 WHERE r.id = ?"
	})
	.bind(record_id)
	.fetch_optional(db_conn.raw_mut())
	.map_err(DatabaseError::from)
	.await
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_detailed_leaderboard(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	#[builder(default = u64::MAX)] size: u64,
) -> impl Stream<Item = DatabaseResult<Record>>
{
	let (conn, query) = db_conn.parts();
	query.reset();

	query.push(format_args! {
		"SELECT
		   r.id,
		   p.id AS player_id,
		   p.name AS player_name,
		   m.id AS map_id,
		   m.name AS map_name,
		   c.id AS course_id,
		   c.local_id AS course_local_id,
		   c.name AS course_name,
		   f.mode,
		   r.styles,
		   r.time,
		   r.teleports,
		   BestRecords.points AS nub_points,
		   BestProRecords.points AS pro_points,
		   r.created_at
		 FROM Records AS r
		 {join_leaderboards}
		 INNER JOIN Players AS p ON p.id = r.player_id
		 INNER JOIN Filters AS f ON f.id = r.filter_id
		 INNER JOIN Courses AS c ON c.id = f.course_id
		 INNER JOIN Maps AS m ON m.id = c.map_id
		 WHERE r.filter_id = ?
		 ORDER BY r.time ASC, r.created_at ASC
		 LIMIT ?",
		join_leaderboards = match leaderboard {
			Leaderboard::NUB => {
				"INNER JOIN BestRecords ON BestRecords.record_id = r.id
				 LEFT JOIN BestProRecords ON BestProRecords.record_id = r.id"
			},
			Leaderboard::PRO => {
				"LEFT JOIN BestRecords ON BestRecords.record_id = r.id
				 INNER JOIN BestProRecords ON BestProRecords.record_id = r.id"
			},
		},
	});

	query
		.build_query_as::<Record>()
		.bind(filter_id)
		.bind(size)
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
pub fn get_leaderboard(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	#[builder(default = u64::MAX)] size: u64,
) -> impl Stream<Item = DatabaseResult<LeaderboardEntry>>
{
	let (conn, query) = db_conn.parts();
	query.reset();

	query.push(format_args! {
		"SELECT
		   r.id,
		   r.player_id,
		   r.time
		 FROM Records AS r
		 INNER JOIN {leaderboard} AS br ON br.record_id = r.id
		 WHERE br.filter_id = ?
		 ORDER BY r.time ASC, r.created_at ASC
		 LIMIT ?",
		leaderboard = match leaderboard {
			Leaderboard::NUB => " BestRecords",
			Leaderboard::PRO => " BestProRecords",
		},
	});

	query
		.build_query_as::<LeaderboardEntry>()
		.bind(filter_id)
		.bind(size)
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[instrument(skip(db_conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
) -> DatabaseResult<u64>
{
	sqlx::query!("DELETE FROM Records LIMIT ?", count)
		.execute(db_conn.raw_mut())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}
