mod id;
mod ip;
mod name;
mod preferences;
mod rating;

use futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt};
use serde::Serialize;
use utoipa::ToSchema;

pub use self::{
	id::{ParsePlayerIdError, PlayerId},
	ip::PlayerIp,
	name::{InvalidPlayerName, PlayerName},
	preferences::PlayerPreferences,
	rating::{InvalidPlayerRating, PlayerRating},
};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	game::Game,
	mode::Mode,
	records::Leaderboard,
	stream::StreamExt as _,
	time::Timestamp,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct Player
{
	pub id: PlayerId,
	pub name: PlayerName,
	pub rating: PlayerRating,
	pub created_at: Timestamp,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: PlayerName,
	#[builder(into)] ip_address: PlayerIp,
) -> DatabaseResult<()>
{
	sqlx::query!(
		"INSERT INTO Players (id, name, ip_address)
		 VALUES (?, ?, ?)
		 ON DUPLICATE KEY
		 UPDATE name = VALUES(name),
		        ip_address = VALUES(ip_address)",
		player_id,
		name,
		ip_address,
	)
	.execute(conn.as_raw())
	.await?;

	Ok(())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: Option<&str>,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Players
		 WHERE name LIKE COALESCE(?, name)",
		name.map(|name| format!("%{name}%")),
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
	name: Option<&str>,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<Player>>
{
	sqlx::query!(
		"SELECT
		   id AS `id: PlayerId`,
		   name AS `name: PlayerName`,
		   rating AS `rating: PlayerRating`,
		   created_at AS `created_at: Timestamp`,
		   MATCH (name) AGAINST (?) AS name_score
		 FROM Players
		 WHERE name LIKE COALESCE(?, name)
		 ORDER BY name_score DESC, created_at DESC
		 LIMIT ?, ?",
		name,
		name.map(|name| format!("%{name}%")),
		offset,
		limit,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
	.map_ok(|row| Player {
		id: row.id,
		name: row.name,
		rating: row.rating,
		created_at: row.created_at,
	})
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<Player>>
{
	sqlx::query_as!(
		Player,
		"SELECT
		   id AS `id: PlayerId`,
		   name AS `name: PlayerName`,
		   rating AS `rating: PlayerRating`,
		   created_at AS `created_at: Timestamp`
		 FROM Players
		 WHERE id = ?",
		player_id,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn is_banned(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query_scalar!(
		"SELECT (COUNT(*) > 0) AS `is_banned: bool`
		 FROM Bans AS b
		 RIGHT JOIN Unbans AS ub ON ub.id = b.id
		 WHERE b.player_id = ?
		 AND (b.id IS NULL OR b.expires_at > NOW())",
		player_id,
	)
	.fetch_one(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_preferences(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	game: Game,
) -> DatabaseResult<Option<PlayerPreferences>>
{
	let (conn, query) = conn.as_parts();

	query.reset();
	query.push("SELECT");
	query.push(match game {
		Game::CS2 => " cs2_preferences ",
		Game::CSGO => " csgo_preferences ",
	});
	query.push("FROM Players WHERE id = ");
	query.push_bind(player_id);

	query
		.build_query_scalar::<PlayerPreferences>()
		.fetch_optional(conn)
		.map_err(DatabaseError::from)
		.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_preferences(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	game: Game,
	preferences: PlayerPreferences,
) -> DatabaseResult<bool>
{
	let (conn, query) = conn.as_parts();

	query.reset();
	query.push("UPDATE Players SET");
	query.push(match game {
		Game::CS2 => " cs2_preferences ",
		Game::CSGO => " csgo_preferences ",
	});
	query.push(" = ");
	query.push_bind(sqlx::types::Json(preferences));
	query.push(" WHERE id = ");
	query.push(player_id);

	query
		.build()
		.execute(conn)
		.map_ok(|query_result| query_result.rows_affected() > 0)
		.map_err(DatabaseError::from)
		.await
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RatingLeaderboardEntry
{
	id: PlayerId,
	name: PlayerName,
	rating: PlayerRating,
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get_rating_leaderboard(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	size: u64,
) -> impl Stream<Item = DatabaseResult<RatingLeaderboardEntry>>
{
	sqlx::query_as!(
		RatingLeaderboardEntry,
		"SELECT
		   id AS `id: PlayerId`,
		   name AS `name: PlayerName`,
		   rating AS `rating: PlayerRating`
		 FROM Players
		 WHERE rating > 0
		 ORDER BY rating DESC
		 LIMIT ?",
		size,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
}

#[derive(Debug, Serialize, sqlx::FromRow, ToSchema)]
pub struct RecordsLeaderboardEntry
{
	id: PlayerId,
	name: PlayerName,
	records: u64,
}

#[tracing::instrument(skip(conn))]
#[builder(finish_fn = exec)]
pub fn get_records_leaderboard(
	#[builder(start_fn)] leaderboard: Leaderboard,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	mode: Option<Mode>,
	size: u64,
) -> impl Stream<Item = DatabaseResult<RecordsLeaderboardEntry>>
{
	let (conn, query) = conn.as_parts();

	query.reset();

	#[rustfmt::skip]
	query.push(r#"
		WITH RecordCounts AS (
		  SELECT r.player_id, COUNT(r.record_id) AS record_count
		  FROM
	"#);

	query.push(match leaderboard {
		Leaderboard::NUB => "BestRecords AS r",
		Leaderboard::PRO => "BestProRecords AS r",
	});

	#[rustfmt::skip]
	query.push(r#"
		  INNER JOIN Filters AS f ON f.id = r.filter_id
		  WHERE r.points = 10000
		  AND f.mode = COALESCE(?, f.mode)
		  GROUP BY r.player_id
		)
		SELECT
		  p.id AS `id: PlayerId`,
		  p.name AS `name: PlayerName`,
		  r.record_count AS `records: u64`
		FROM RecordCounts AS r
		INNER JOIN Players AS p ON p.id = r.player_id
		ORDER BY r.record_count DESC
		LIMIT ?
	"#);

	query
		.build_query_as::<RecordsLeaderboardEntry>()
		.bind(mode)
		.bind(size)
		.fetch(conn)
		.map_err(DatabaseError::from)
		.fuse()
		.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn recalculate_rating(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<PlayerRating>>
{
	sqlx::query!(
		"UPDATE PlayersToRecalculate
		 SET priority = 0
		 WHERE player_id = ?",
		player_id,
	)
	.execute(conn.as_raw())
	.await?;

	let maybe_rating = sqlx::query_scalar!(
		"WITH RankedPoints AS (
		   SELECT
		     player_id,
		     leaderboard,
		     points,
		     ROW_NUMBER() OVER (
		       PARTITION BY player_id
		       ORDER BY
		         points DESC,
		         leaderboard DESC
		     ) AS n
		   FROM ((
		     SELECT 1 AS leaderboard, points, player_id
		     FROM BestRecords
		     WHERE player_id = ?
		   ) UNION ALL (
		     SELECT 2 AS leaderboard, points, player_id
		     FROM BestProRecords
		     WHERE player_id = ?
		   )) AS _
		 )
		 SELECT SUM(points * POWER(0.975, n - 1)) AS `rating: PlayerRating`
		 FROM RankedPoints
         GROUP BY player_id",
		player_id,
		player_id,
	)
	.fetch_optional(conn.as_raw())
	.await?
	.flatten();

	if let Some(rating) = maybe_rating {
		sqlx::query!("UPDATE Players SET rating = ? WHERE id = ?", rating, player_id)
			.execute(conn.as_raw())
			.await?;
	}

	Ok(maybe_rating)
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<u64>
{
	sqlx::query!("DELETE FROM Players LIMIT ?", count)
		.execute(conn.as_raw())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}
