use std::time::Duration;

use time::OffsetDateTime;

use crate::database;
use crate::users::sessions::SessionID;
use crate::users::{self, UserID};

#[instrument(skip(conn), ret(level = "debug"), err(level = "debug"))]
pub async fn create(
	conn: &mut database::Connection,
	user_id: UserID,
	expires_after: Duration,
) -> Result<SessionID, CreateSessionError> {
	let session_id = SessionID::new();
	let expires_at = OffsetDateTime::now_utc() + expires_after;

	match users::database::create(conn, user_id).await {
		Ok(()) => {
			debug!(?user_id, "user did not exist yet; created new account");
		}
		Err(users::database::CreateUserError::DuplicateUserID) => {}
		Err(users::database::CreateUserError::Database(error)) => {
			return Err(CreateSessionError::Database(error));
		}
	}

	sqlx::query! {
		"INSERT INTO UserSessions (id, user_id, expires_at)
		 VALUES (?, ?, ?)",
		session_id,
		user_id,
		expires_at,
	}
	.execute(conn)
	.await?;

	Ok(session_id)
}

#[derive(Debug, Error)]
pub enum CreateSessionError {
	#[error("user does not exist")]
	UserDoesNotExist,

	#[error(transparent)]
	Database(#[from] database::Error),
}
