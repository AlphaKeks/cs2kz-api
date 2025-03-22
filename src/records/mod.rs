mod id;
mod points;
mod rank;
mod time;

use futures_util::{Stream, StreamExt as _, TryStreamExt};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub use self::{
	id::{ParseRecordIdError, RecordId},
	points::{InvalidPoints, Points},
	rank::Rank,
	time::{InvalidTime, Time},
};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	maps::FilterId,
	players::{PlayerId, PlayerName},
	stream::StreamExt as _,
	time::Timestamp,
};

#[derive(Debug, Serialize, ToSchema, sqlx::FromRow)]
pub struct LeaderboardEntry
{
	pub id: RecordId,
	#[sqlx(flatten)]
	pub player: PlayerInfo,
	pub time: Time,
	pub teleports: u32,
	pub points: Points,
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

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Leaderboard
{
	NUB,
	PRO,
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
		   r.id AS `id: RecordId`,
		   p.id AS `player_id: PlayerId`,
		   p.name AS `player_name: PlayerName`,
		   r.time AS `time: Time`,
		   r.teleports,
		   br.points AS `points: Points`
		 FROM Records AS r
		 INNER JOIN"
	});

	query.push(match leaderboard {
		Leaderboard::NUB => " BestRecords AS br ON br.record_id = r.id WHERE br.filter_id = ",
		Leaderboard::PRO => " BestProRecords AS br ON br.record_id = r.id WHERE br.filter_id = ",
	});

	query.push_bind(filter_id);
	query.push(" INNER JOIN Players AS p ON p.id = r.player_id");
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
