mod id;
mod name;
mod permissions;
mod server_budget;
pub mod sessions;

use std::num::NonZero;

use futures_util::{Stream, StreamExt as _, TryFutureExt, TryStreamExt};
use serde::Serialize;
use utoipa::ToSchema;

pub use self::{
	id::{ParseUserIdError, UserId},
	name::{InvalidUsername, Username},
	permissions::{Iter as PermissionsIter, Permission, Permissions},
	server_budget::ServerBudget,
};
use crate::{
	database::{DatabaseConnection, DatabaseError, DatabaseResult},
	email::EmailAddress,
	stream::StreamExt as _,
	time::Timestamp,
};

#[derive(Debug, Serialize, ToSchema)]
pub struct User
{
	pub id: UserId,
	pub name: Username,
	pub permissions: Permissions,
	pub created_at: Timestamp,
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn create(
	#[builder(start_fn)] user_id: UserId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: Username,
) -> DatabaseResult<()>
{
	sqlx::query!(
		"INSERT INTO Users (id, name)
		 VALUES (?, ?)
		 ON DUPLICATE KEY
		 UPDATE name = VALUES(name)",
		user_id,
		name,
	)
	.execute(conn.as_raw())
	.await?;

	Ok(())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn count(
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(default)] has_permissions: bool,
	#[builder(default)] required_permissions: Permissions,
) -> DatabaseResult<u64>
{
	sqlx::query_scalar!(
		"SELECT COUNT(*)
		 FROM Users
		 WHERE permissions >= ?
		 AND (permissions & ?) = ?",
		has_permissions,
		required_permissions,
		required_permissions,
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
	#[builder(default)] has_permissions: bool,
	#[builder(default)] required_permissions: Permissions,
	#[builder(default = 0)] offset: u64,
	limit: u64,
) -> impl Stream<Item = DatabaseResult<User>>
{
	sqlx::query_as!(
		User,
		"SELECT
		   id AS `id: UserId`,
		   name AS `name: Username`,
		   permissions AS `permissions: Permissions`,
		   created_at AS `created_at: Timestamp`
		 FROM Users
		 WHERE permissions >= ?
		 AND (permissions & ?) = ?
		 ORDER BY created_at ASC
		 LIMIT ?, ?",
		has_permissions,
		required_permissions,
		required_permissions,
		offset,
		limit,
	)
	.fetch(conn.as_raw())
	.map_err(DatabaseError::from)
	.fuse()
	.instrumented(tracing::Span::current())
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn get_by_id(
	#[builder(start_fn)] user_id: UserId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<Option<User>>
{
	sqlx::query_as!(
		User,
		"SELECT
		   id AS `id: UserId`,
		   name AS `name: Username`,
		   permissions AS `permissions: Permissions`,
		   created_at AS `created_at: Timestamp`
		 FROM Users
		 WHERE id = ?",
		user_id,
	)
	.fetch_optional(conn.as_raw())
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn decrement_server_budget(
	#[builder(start_fn)] user_id: UserId,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	#[builder(default = NonZero::<u16>::MIN)] amount: NonZero<u16>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET server_budget = server_budget - ?
		 WHERE id = ?",
		amount,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_email(
	#[builder(start_fn)] user_id: UserId,
	#[builder(start_fn)] email: Option<EmailAddress>,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET email_address = ?
		 WHERE id = ?",
		email,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_permissions(
	#[builder(start_fn)] user_id: UserId,
	#[builder(start_fn)] permissions: Permissions,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET permissions = ?
		 WHERE id = ?",
		permissions,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn add_permissions(
	#[builder(start_fn)] user_id: UserId,
	#[builder(start_fn, into)] permissions: Permissions,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET permissions = (permissions | ?)
		 WHERE id = ?",
		permissions,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn remove_permissions(
	#[builder(start_fn)] user_id: UserId,
	#[builder(start_fn, into)] permissions: Permissions,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET permissions = (permissions & (~(?)))
		 WHERE id = ?",
		permissions,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn set_server_budget(
	#[builder(start_fn)] user_id: UserId,
	#[builder(start_fn)] budget: ServerBudget,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<bool>
{
	sqlx::query!(
		"UPDATE Users
		 SET server_budget = ?
		 WHERE id = ?",
		budget,
		user_id,
	)
	.execute(conn.as_raw())
	.map_ok(|query_result| query_result.rows_affected() > 0)
	.map_err(DatabaseError::from)
	.await
}

#[tracing::instrument(skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
pub async fn delete(
	#[builder(start_fn)] count: u64,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
) -> DatabaseResult<u64>
{
	sqlx::query!("DELETE FROM Users LIMIT ?", count)
		.execute(conn.as_raw())
		.map_ok(|query_result| query_result.rows_affected())
		.map_err(DatabaseError::from)
		.await
}
