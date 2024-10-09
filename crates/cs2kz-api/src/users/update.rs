use crate::events::Event;
use crate::users::{UserID, UserUpdate};
use crate::{database, events, users};

#[instrument(skip(pool), err(level = "debug"))]
pub async fn update(
	pool: &database::ConnectionPool,
	update: UserUpdate,
) -> Result<(), UpdateUserError> {
	let mut conn = pool.acquire().await?;

	users::database::update(&mut conn, &update).await?;
	events::dispatch(Event::UserUpdated(update));

	Ok(())
}

#[derive(Debug, Error)]
pub enum UpdateUserError {
	#[error("a user with that ID does not exist")]
	UserDoesNotExist,

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(users::database::UpdateUserError => UpdateUserError => {
	E::UserDoesNotExist => Self::UserDoesNotExist,
	E::Database(source) => Self::Database(source),
});
