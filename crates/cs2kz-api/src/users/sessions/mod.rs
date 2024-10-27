//! User sessions.

use futures_util::TryStreamExt;

use crate::database;
use crate::database::RowStream;
use crate::time::Timestamp;
use crate::users::{Permissions, UserID};

mod session_id;
pub use session_id::{ParseSessionIDError, SessionID};

mod session;
pub use session::{Session, SessionRejection, UserInfo};

pub mod authorization;

/// Information about a session.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct SessionInfo {
	/// When this session was created.
	pub created_at: Timestamp,

	/// When this session will expire.
	pub expires_at: Timestamp,
}

/// Returns detailed information about a given session.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn get_by_id(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<Option<SessionInfo>> {
	sqlx::query_as!(
		SessionInfo,
		"SELECT
		   s.created_at `created_at: Timestamp`,
		   s.expires_at `expires_at: Timestamp`
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE s.id = ?
		 AND s.expires_at > NOW()",
		session_id,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Information about the user associated with a specific session.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn get_user_info(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<Option<UserInfo>> {
	Ok(sqlx::query!(
		"SELECT
		   u.id `user_id: UserID`,
		   u.permissions `user_permissions: Permissions`
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE s.id = ?
		 AND s.expires_at > NOW()",
		session_id,
	)
	.fetch_optional(conn.as_mut())
	.await?
	.map(|row| UserInfo::new(row.user_id, row.user_permissions)))
}

/// Returns all (not-yet-expired) sessions that belong to a specific user.
#[instrument(level = "debug", skip(conn))]
pub fn get_by_user(
	conn: &mut database::Connection,
	user_id: UserID,
) -> impl RowStream<'_, SessionInfo> {
	sqlx::query_as!(
		SessionInfo,
		"SELECT
		   s.created_at `created_at: Timestamp`,
		   s.expires_at `expires_at: Timestamp`
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE u.id = ?
		 AND s.expires_at > NOW()",
		user_id,
	)
	.fetch(conn.as_mut())
	.map_err(Into::into)
}

/// Invalidates the session with the given ID.
///
/// Returns whether anything has actually changed, as already-expired sessions
/// will not be updated again.
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
	.execute(conn.as_mut())
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 row ({n})"),
	})
	.map_err(Into::into)
}

/// Invalidates all (not-yet-expired) sessions for a given user.
///
/// Returns whether anything has actually changed, as already-expired sessions
/// will not be updated again.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn invalidate_for_user(
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
	.execute(conn.as_mut())
	.await
	.map(|result| result.rows_affected() > 0)
	.map_err(Into::into)
}
