//! This module implements functionality to update players.

use std::iter;

use cs2kz::{Mode, SteamID};
use problem_details::AsProblemDetails;
use serde::Deserialize;
use sqlx::Row;

use super::{PlayerService, Preferences};
use crate::database;
use crate::http::Problem;
use crate::services::maps::CourseID;
use crate::services::servers::ServerID;
use crate::stats::{CourseSessionData, CourseSessionID, GameSession, GameSessionID};
use crate::util::net::IpAddr;

#[expect(missing_docs)]
pub type Result<T = Response, E = Error> = std::result::Result<T, E>;

impl PlayerService
{
	/// Updates a player.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn update_player(
		&self,
		player_id: SteamID,
		server_id: ServerID,
		request: Request,
	) -> Result
	{
		let mut txn = self.mysql.begin().await?;

		sqlx::query! {
			"UPDATE Users
			 SET name = ?,
			     ip_address = ?,
			     game_preferences = ?,
			     last_seen_on = NOW()
			 WHERE id = ?",
			request.name,
			request.ip_address,
			database::Json(request.preferences),
			player_id,
		}
		.execute(txn.as_mut())
		.await?;

		info!("updated player metadata");

		let game_session_id =
			insert_game_session(player_id, server_id, &request.session, &mut txn).await?;

		info!(%game_session_id, "created game session");

		for (course_id, (mode, session)) in request
			.session
			.course_sessions
			.iter()
			.flat_map(|(&course_id, session)| iter::zip(iter::repeat(course_id), session.iter()))
		{
			let course_session_id =
				insert_course_session(game_session_id, course_id, mode, session, &mut txn).await?;

			info!(%course_id, %mode, %course_session_id, "created course session");
		}

		txn.commit().await?;

		Ok(())
	}
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn insert_game_session(
	player_id: SteamID,
	server_id: ServerID,
	session: &GameSession,
	txn: &mut database::Transaction<'_>,
) -> sqlx::Result<GameSessionID>
{
	sanity_check!(session.is_valid(), "valid game session");

	sqlx::query! {
		"INSERT INTO GameSessions (
		   user_id,
		   server_id,
		   time_active,
		   time_spectating,
		   time_afk,
		   bhops,
		   perfs
		 )
		 VALUES
		   (?, ?, ?, ?, ?, ?, ?)
		 RETURNING id",
		player_id,
		server_id,
		session.seconds_active,
		session.seconds_spectating,
		session.seconds_afk,
		session.bhop_stats.total,
		session.bhop_stats.perfs,
	}
	.fetch_one(txn.as_mut())
	.await
	.and_then(|row| row.try_get(0))
}

#[instrument(level = "debug", err(Debug, level = "debug"), skip(txn))]
async fn insert_course_session(
	game_session_id: GameSessionID,
	course_id: CourseID,
	mode: Mode,
	session: &CourseSessionData,
	txn: &mut database::Transaction<'_>,
) -> sqlx::Result<CourseSessionID>
{
	sanity_check!(session.is_valid(), "valid course session");

	sqlx::query! {
		"INSERT INTO CourseSessions (
		   game_session_id,
		   course_id,
		   game_mode,
		   playtime,
		   bhops,
		   perfs,
		   started_runs,
		   finished_runs
		 )
		 VALUES
		   (?, ?, ?, ?, ?, ?, ?, ?)
		 RETURNING id",
		game_session_id,
		course_id,
		mode,
		session.playtime,
		session.bhop_stats.total,
		session.bhop_stats.perfs,
		session.started_runs,
		session.finished_runs,
	}
	.fetch_one(txn.as_mut())
	.await
	.and_then(|row| row.try_get(0))
}

/// Request for updating a player.
#[derive(Debug, Deserialize)]
pub struct Request
{
	/// The player's current name.
	pub name: String,

	/// The player's current IP address.
	pub ip_address: IpAddr,

	/// The player's current in-game preferences.
	pub preferences: Preferences,

	/// Session information.
	pub session: GameSession,
}

/// Response for updating a player.
pub type Response = ();

/// Errors that can occur when getting players.
#[expect(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
	#[error("player not found")]
	PlayerNotFound,

	#[error("something went wrong; please report this incident")]
	Database(#[from] sqlx::Error),
}

impl AsProblemDetails for Error
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::PlayerNotFound => Problem::ResourceNotFound,
			Self::Database(_) => Problem::Internal,
		}
	}
}

impl_into_response!(Error);
