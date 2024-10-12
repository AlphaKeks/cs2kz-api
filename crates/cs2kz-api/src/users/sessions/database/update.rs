use std::time::Duration;

use time::OffsetDateTime;

use crate::database;
use crate::users::sessions::SessionID;
use crate::users::UserID;

#[instrument(skip(conn), err(level = "debug"))]
pub async fn extend_session(
	conn: &mut database::Connection,
	session_id: SessionID,
	duration: Duration,
) -> database::Result<()> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = ?
		 WHERE id = ?
		 AND expires_at > NOW()",
		OffsetDateTime::now_utc() + duration,
		session_id,
	)
	.execute(conn)
	.await?;

	Ok(())
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn invalidate_session(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<()> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = NOW()
		 WHERE id = ?
		 AND expires_at > NOW()",
		session_id,
	)
	.execute(conn)
	.await?;

	Ok(())
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn invalidate_sessions(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<()> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = NOW()
		 WHERE user_id = ?
		 AND expires_at > NOW()",
		user_id,
	)
	.execute(conn)
	.await?;

	Ok(())
}
