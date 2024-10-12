use crate::events::Event;
use crate::users::UserID;
use crate::{database, events, users};

#[instrument(skip(pool), err(level = "debug"))]
pub async fn register(
	pool: &database::ConnectionPool,
	user_id: UserID,
) -> Result<(), RegisterUserError> {
	let mut txn = pool.begin().await?;

	users::database::create(&mut txn, user_id).await?;
	txn.commit().await?;
	events::dispatch(Event::UserRegistered { id: user_id });

	Ok(())
}

#[derive(Debug, Error)]
pub enum RegisterUserError {
	#[error("a user with that ID already exists")]
	UserAlreadyExists,

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(users::database::CreateUserError => RegisterUserError => {
	E::DuplicateUserID => Self::UserAlreadyExists,
	E::Database(source) => Self::Database(source),
});
