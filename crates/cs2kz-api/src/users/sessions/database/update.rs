use std::time::Duration;

use crate::database;
use crate::time::Timestamp;
use crate::users::sessions::SessionID;
use crate::users::UserID;

pub struct ExtendSession {
	pub session_id: SessionID,
	pub duration: Duration,
}

/// Extends a session by the given duration.
///
/// The returned boolean indicates whether a user has actually been updated.
/// If this is `false`, it means there is no session with an ID of `session_id`.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn extend(
	conn: &mut database::Connection,
	ExtendSession {
		session_id,
		duration,
	}: ExtendSession,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = ?
		 WHERE id = ?",
		Timestamp::now() + duration,
		session_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 session ({n})"),
	})
	.map_err(Into::into)
}

/// Invalidates a specific session by setting its expiration date to "now".
///
/// The returned boolean indicates whether a row has actually been updated.
/// If this is `false`, it means there is no session with an ID of `session_id`.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn invalidate(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = NOW()
		 WHERE id = ?
		 AND expires_at > NOW()",
		session_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 session ({n})"),
	})
	.map_err(Into::into)
}

/// Invalidates all sessions of a given user by setting their expiration dates to "now".
///
/// The returned boolean indicates whether any rows have actually been updated.
/// If this is `false`, it means the user does not currently have any active sessions.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn invalidate_all(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = NOW()
		 WHERE user_id = ?
		 AND expires_at > NOW()",
		user_id,
	)
	.execute(conn)
	.await
	.map(|result| result.rows_affected() > 0)
	.map_err(Into::into)
}
