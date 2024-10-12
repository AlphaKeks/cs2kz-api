use super::Session;
use crate::database;
use crate::users::permissions::Permissions;
use crate::users::sessions::SessionID;
use crate::users::UserID;

#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	session_id: SessionID,
) -> database::Result<Option<Session>> {
	sqlx::query_as!(
		Session,
		"SELECT
		   s.id `id: SessionID`,
		   u.id `user_id: UserID`,
		   u.permissions `user_permissions: Permissions`,
		   s.created_at,
		   s.expires_at
		 FROM UserSessions s
		 JOIN Users u ON u.id = s.user_id
		 WHERE s.id = ?
		 AND s.expires_at > NOW()",
		session_id,
	)
	.fetch_optional(conn)
	.await
}
