mod id;
mod points;
mod rank;
mod teleports;
mod time;

use futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use utoipa::ToSchema;

pub use self::{
	id::{ParseRecordIdError, RecordId},
	points::{InvalidPoints, Points},
	rank::Rank,
	teleports::Teleports,
	time::{InvalidTime, Time},
};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult, QueryBuilder},
	maps::{CourseId, CourseLocalId, CourseName, FilterId, MapId, MapName, Tier},
	mode::Mode,
	players::{self, PlayerId, PlayerName, PlayerRating},
	points::CalculateLeaderboardPortionForNewRecordError,
	servers::ServerSessionId,
	stream::StreamExt as _,
	styles::Styles,
	time::Timestamp,
};

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
	Database(DatabaseError),
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	session_id: ServerSessionId,
	time: Time,
	teleports: Teleports,
	styles: Styles,
) -> Result<CreatedRecord, CreateRecordError>
{
	#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
	#[builder(finish_fn = exec)]
	async fn update_leaderboard(
		#[builder(start_fn)] filter_id: FilterId,
		#[builder(start_fn)] leaderboard: Leaderboard,
		#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
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
				.fetch_one(conn.as_raw())
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
				.fetch_all(conn.as_raw())
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
						.exec(&mut *conn)
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

	#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
	#[builder(finish_fn = exec)]
	async fn update_pb(
		#[builder(start_fn)] filter_id: FilterId,
		#[builder(start_fn)] player_id: PlayerId,
		#[builder(start_fn)] record_id: RecordId,
		#[builder(start_fn)] leaderboard: Leaderboard,
		#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
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
			.execute(conn.as_raw())
			.await?;

		Ok(())
	}

	if players::is_banned(player_id).exec(&mut *conn).await? {
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
	.fetch_one(conn.as_raw())
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
	.fetch_optional(conn.as_raw())
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
	.fetch_optional(conn.as_raw())
	.await?;

	let nub_stats = if nub_pb.as_ref().is_none_or(|pb| pb.time > time) {
		let stats = update_leaderboard(filter_id, Leaderboard::NUB)
			.time(time)
			.has_pb(nub_pb.is_some())
			.exec(&mut *conn)
			.await?;

		update_pb(filter_id, player_id, record_id, Leaderboard::NUB)
			.points(stats.points)
			.exec(&mut *conn)
			.await?;

		Some(stats)
	} else {
		None
	};

	let pro_stats = if teleports.as_u32() == 0 && pro_pb.as_ref().is_none_or(|pb| pb.time > time) {
		let stats = update_leaderboard(filter_id, Leaderboard::PRO)
			.time(time)
			.has_pb(nub_pb.is_some())
			.exec(&mut *conn)
			.await?;

		update_pb(filter_id, player_id, record_id, Leaderboard::PRO)
			.points(stats.points)
			.exec(&mut *conn)
			.await?;

		Some(stats)
	} else {
		None
	};

	let player_rating = if nub_stats.is_some() || pro_stats.is_some() {
		players::recalculate_rating(player_id).exec(&mut *conn).await?
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

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	player: Option<PlayerId>,
	course: Option<CourseId>,
	mode: Option<Mode>,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(DISTINCT r.id)
		 FROM Records AS r
		 INNER JOIN Filters AS f ON f.id = r.filter_id
		 WHERE r.player_id = COALESCE(?, r.player_id)
		 AND f.course_id = COALESCE(?, f.course_id)
		 AND f.mode = COALESCE(?, f.mode)",
		player,
		course,
		mode,
	)
	.fetch_one(conn.as_raw())
	.map_err(DatabaseError::from)
	.and_then(async |row| row.try_into().map_err(DatabaseError::convert_count))
	.await
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	player: Option<PlayerId>,
	course: Option<CourseId>,
	mode: Option<Mode>,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Record>>
{
	let (conn, query) = conn.as_parts();
	query.reset();

	query.push({
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
		 WHERE r.player_id = COALESCE(?, r.player_id)
		 AND c.id = COALESCE(?, c.id)
		 AND f.mode = COALESCE(?, f.mode)
		 ORDER BY r.id DESC
		 LIMIT ?, ?"
	});

	query
		.build_query_as::<Record>()
		.bind(player)
		.bind(course)
		.bind(mode)
		.bind(offset)
		.bind(limit)
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] record_id: RecordId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Record>>
{
	let (conn, query) = conn.as_parts();
	query.reset();

	query.push({
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
	});

	query
		.build_query_as::<Record>()
		.bind(record_id)
		.fetch_optional(conn)
		.map_err(DatabaseError::from)
		.await
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get_detailed_leaderboard(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(default = u64::MAX)] size: u64,
) -> impl Stream<Item = DatabaseResult<Record>>
{
	let (conn, query) = conn.as_parts();
	query.reset();

	query.push({
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
		 FROM Records AS r"
	});

	match leaderboard {
		Leaderboard::NUB => {
			query.push({
				" INNER JOIN BestRecords ON BestRecords.record_id = r.id
				  LEFT JOIN BestProRecords ON BestProRecords.record_id = r.id"
			});
		},
		Leaderboard::PRO => {
			query.push({
				" LEFT JOIN BestRecords ON BestRecords.record_id = r.id
				  INNER JOIN BestProRecords ON BestProRecords.record_id = r.id"
			});
		},
	}

	query.push(" INNER JOIN Players AS p ON p.id = r.player_id ");
	query.push(" INNER JOIN Filters AS f ON f.id = r.filter_id ");
	query.push(" INNER JOIN Courses AS c ON c.id = f.course_id ");
	query.push(" INNER JOIN Maps AS m ON m.id = c.map_id ");
	query.push(" WHERE r.filter_id = ?");
	query.push(" ORDER BY r.time ASC, r.created_at ASC ");
	query.push(" LIMIT ? ");

	query
		.build_query_as::<Record>()
		.bind(filter_id)
		.bind(size)
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get_leaderboard(
	#[builder(start_fn)] filter_id: FilterId,
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(default = u64::MAX)] size: u64,
) -> impl Stream<Item = DatabaseResult<LeaderboardEntry>>
{
	let (conn, query) = conn.as_parts();
	query.reset();

	query.push({
		"SELECT
		   r.id,
		   r.player_id,
		   r.time
		 FROM Records AS r
		 INNER JOIN"
	});

	query.push(match leaderboard {
		Leaderboard::NUB => " BestRecords AS br ON br.record_id = r.id WHERE br.filter_id = ",
		Leaderboard::PRO => " BestProRecords AS br ON br.record_id = r.id WHERE br.filter_id = ",
	});

	query.push_bind(filter_id);
	query.push(" ORDER BY r.time ASC, r.created_at ASC");
	query.push(" LIMIT ");
	query.push_bind(size);

	query
		.build_query_as::<LeaderboardEntry>()
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<u64>
{
	sqlx::query!("DELETE FROM Records LIMIT ?", count)
		.execute(conn.as_raw())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}
