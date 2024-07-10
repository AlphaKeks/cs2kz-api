//! Everything related to KZ game sessions.

use axum::extract::FromRef;
use sqlx::{MySql, Pool};

use crate::{Error, Result};

mod models;
pub use models::{CourseSessionID, GameSession, GameSessionID, TimeSpent};

pub mod http;

/// A service for dealing with KZ game sessions as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct GameSessionService
{
	database: Pool<MySql>,
}

impl GameSessionService
{
	/// Creates a new [`GameSessionService`] instance.
	pub const fn new(database: Pool<MySql>) -> Self
	{
		Self { database }
	}

	/// Fetches a single game session.
	pub async fn fetch_session(&self, session_id: GameSessionID) -> Result<GameSession>
	{
		let session = sqlx::query_as(
			r#"
			SELECT
			  s.id,
			  p.name player_name,
			  p.id player_id,
			  sv.name server_name,
			  sv.id server_id,
			  s.time_active,
			  s.time_spectating,
			  s.time_afk,
			  s.bhops,
			  s.perfs,
			  s.created_on
			FROM
			  GameSessions s
			  JOIN Players p ON p.id = s.player_id
			  JOIN Servers sv ON sv.id = s.server_id
			WHERE
			  s.id = ?
			"#,
		)
		.bind(session_id)
		.fetch_optional(&self.database)
		.await?
		.ok_or_else(|| Error::not_found("session"))?;

		Ok(session)
	}
}
