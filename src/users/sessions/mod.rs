mod id;

use std::{fmt, ops, time::Duration};

use futures_util::TryFutureExt;

pub use self::id::{ParseSessionIdError, SessionId, SessionIdRejection};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	time::Timestamp,
	users::{Permissions, ServerBudget, UserId, Username},
};

#[derive(Debug)]
pub struct Session
{
	pub id: SessionId,
	pub user: User,
	pub expires_at: Timestamp,
}

#[derive(Debug)]
pub struct User
{
	pub id: UserId,
	pub name: Username,
	pub permissions: Permissions,
	pub server_budget: ServerBudget,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
pub async fn get_by_id(
	conn: &mut DatabaseConnection<'_, '_>,
	session_id: SessionId,
) -> DatabaseResult<Option<Session>>
{
	sqlx::query!(
		"SELECT
		   s.id AS `id: SessionId`,
		   s.expires_at AS `expires_at: Timestamp`,
		   u.id AS `user_id: UserId`,
		   u.name AS `username: Username`,
		   u.permissions AS `user_permissions: Permissions`,
		   u.server_budget AS `user_server_budget: ServerBudget`
		 FROM UserSessions AS s
		 INNER JOIN Users AS u ON u.id = s.user_id
		 WHERE s.id = ?",
		session_id,
	)
	.fetch_optional(conn.as_raw())
	.await
	.map_err(DatabaseError::from)
	.map(|maybe_row| {
		maybe_row.map(|row| Session {
			id: row.id,
			user: User {
				id: row.user_id,
				name: row.username,
				permissions: row.user_permissions,
				server_budget: row.user_server_budget,
			},
			expires_at: row.expires_at,
		})
	})
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create<Duration>(
	#[builder(start_fn)] user_id: UserId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	expires_after: Duration,
) -> DatabaseResult<SessionId>
where
	Timestamp: ops::Add<Duration, Output = Timestamp>,
	Duration: fmt::Debug,
{
	let session_id = SessionId::new();

	sqlx::query!(
		"INSERT INTO UserSessions (id, user_id, expires_at)
		 VALUES (?, ?, ?)",
		session_id,
		user_id,
		Timestamp::now() + expires_after,
	)
	.execute(conn.as_raw())
	.await?;

	Ok(session_id)
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn extend(
	#[builder(start_fn)] session_id: SessionId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(into)] duration: Duration,
) -> DatabaseResult<bool>
{
	set_expires_at(conn, session_id, Timestamp::now() + duration).await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn expire(
	#[builder(start_fn)] session_id: SessionId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	set_expires_at(conn, session_id, Timestamp::now()).await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn expire_active(
	#[builder(start_fn)] user_id: UserId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE UserSessions
		 SET expires_at = NOW()
		 WHERE user_id = ? AND expires_at > NOW()",
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

fn set_expires_at(
	conn: &mut DatabaseConnection<'_, '_>,
	session_id: SessionId,
	expires_at: Timestamp,
) -> impl Future<Output = DatabaseResult<bool>>
{
	sqlx::query!("UPDATE UserSessions SET expires_at = ? WHERE id = ?", expires_at, session_id)
		.execute(conn.as_raw())
		.map_ok(|query_result| query_result.rows_affected() > 0)
		.map_err(DatabaseError::from)
}
