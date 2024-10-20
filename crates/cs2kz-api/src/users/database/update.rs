use crate::database::{self};
use crate::email::EmailAddress;
use crate::users::permissions::Permissions;
use crate::users::UserID;

pub struct UserUpdate<'a> {
	pub user_id: UserID,
	pub permissions: Option<Permissions>,
	pub email: Option<UpdateEmailAddress<'a>>,
}

#[derive(Debug)]
pub enum UpdateEmailAddress<'a> {
	Clear,
	Set(&'a EmailAddress),
}

/// Updates a user's `last_seen_at` column to "now".
///
/// The returned boolean indicates whether a row has actually been updated.
/// If this is `false`, it means there is no user with an ID of `user_id`.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn mark_as_seen(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE Users
		 SET last_seen_at = NOW()
		 WHERE id = ?",
		user_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 user ({n})"),
	})
	.map_err(Into::into)
}

/// Updates a user.
///
/// The returned boolean indicates whether a row has actually been updated.
/// If this is `false`, it means there is no user with an ID of `user_id`.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn update(
	conn: &mut database::Connection,
	UserUpdate {
		user_id,
		permissions,
		email,
	}: UserUpdate<'_>,
) -> database::Result<bool> {
	sqlx::query!(
		"UPDATE Users
		 SET permissions = COALESCE(?, permissions),
		     email = ?
		 WHERE id = ?",
		permissions,
		email.map(|update| match update {
			UpdateEmailAddress::Clear => "",
			UpdateEmailAddress::Set(email) => email.as_str(),
		}),
		user_id,
	)
	.execute(conn)
	.await
	.map(|result| match result.rows_affected() {
		0 => false,
		1 => true,
		n => panic!("updated more than 1 user ({n})"),
	})
	.map_err(Into::into)
}
