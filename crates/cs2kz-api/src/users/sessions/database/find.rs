use futures::stream::TryStreamExt;

use crate::database::{self, RowStream};
use crate::time::Timestamp;
use crate::users::permissions::Permissions;
use crate::users::sessions::SessionID;
use crate::users::UserID;

/// A row in the `UserSessions` table.
#[derive(Debug)]
pub struct Session {
	pub id: SessionID,
	pub user_id: UserID,
	pub user_permissions: Permissions,
	pub created_at: Timestamp,
	pub expires_at: Timestamp,
}

/// Finds a specific session by its ID.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn find(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<Option<Session>> {
	sqlx::query_as!(
		Session,
		"SELECT
		   s.id `id: SessionID`,
		   u.id `user_id: UserID`,
		   u.permissions `user_permissions: Permissions`,
		   s.created_at `created_at: Timestamp`,
		   s.expires_at `expires_at: Timestamp`
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE s.id = ?",
		session_id,
	)
	.fetch_optional(conn)
	.await
	.map_err(Into::into)
}

/// Returns a [`Stream`] of [`Sessions`]s that belong to a given user.
pub fn find_by_user(
	conn: &mut database::Connection,
	user_id: UserID,
) -> impl RowStream<'_, Session> {
	sqlx::query_as!(
		Session,
		"SELECT
		   s.id `id: SessionID`,
		   u.id `user_id: UserID`,
		   u.permissions `user_permissions: Permissions`,
		   s.created_at `created_at: Timestamp`,
		   s.expires_at `expires_at: Timestamp`
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE u.id = ?
		 AND s.expires_at > NOW()",
		user_id,
	)
	.fetch(conn)
	.map_err(Into::into)
	.inspect_err(|error| error!(%error, "failed to decode row"))
}
