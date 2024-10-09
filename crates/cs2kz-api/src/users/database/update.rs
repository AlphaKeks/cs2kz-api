use cs2kz::SteamID;

use crate::database;
use crate::users::email::Email;
use crate::users::permissions::Permissions;
use crate::users::UserID;

#[instrument(skip(conn), err(level = "debug"))]
pub async fn update(
	conn: &mut database::Connection,
	UserUpdate {
		user_id,
		permissions,
		email,
	}: &UserUpdate,
) -> Result<(), UpdateUserError> {
	let email = email.as_ref().map(|update| match update {
		EmailUpdate::Clear => "",
		EmailUpdate::NewEmail(email) => email.as_str(),
	});

	let query_result = sqlx::query! {
		"UPDATE Users
		 SET permissions = COALESCE(?, permissions),
		     email = COALESCE(?, email),
		     last_seen_at = NOW()
		 WHERE id = ?",
		permissions,
		email,
		user_id,
	}
	.execute(conn)
	.await?;

	if query_result.rows_affected() == 0 {
		return Err(UpdateUserError::UserDoesNotExist);
	}

	Ok(())
}

#[derive(Debug, Clone)]
pub struct UserUpdate {
	pub user_id: UserID,
	pub permissions: Option<Permissions>,
	pub email: Option<EmailUpdate>,
}

#[derive(Debug, Clone)]
pub enum EmailUpdate {
	/// Clear their email.
	Clear,

	/// Change their email to this new one.
	NewEmail(Email),
}

#[derive(Debug, Error)]
pub enum UpdateUserError {
	#[error("there is no user with the given ID")]
	UserDoesNotExist,

	#[error(transparent)]
	Database(#[from] database::Error),
}
