use crate::events::Event;
use crate::users::email::Email;
use crate::users::permissions::Permissions;
use crate::users::UserID;
use crate::{database, events, users};

#[instrument(skip(pool), err(level = "debug"))]
pub async fn update(
	pool: &database::ConnectionPool,
	update: UserUpdate,
) -> Result<(), UpdateUserError> {
	let mut txn = pool.begin().await?;

	users::database::update(&mut txn, users::database::UserUpdate {
		user_id: update.user_id,
		permissions: update.permissions,
		email: update.email.as_ref().map(|update| match update {
			EmailUpdate::Clear => users::database::EmailUpdate::Clear,
			EmailUpdate::NewEmail(email) => users::database::EmailUpdate::NewEmail(email),
		}),
	})
	.await?;

	txn.commit().await?;
	events::dispatch(Event::UserUpdated(update));

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
	#[error("a user with that ID does not exist")]
	UserDoesNotExist,

	#[error("database error: {0}")]
	Database(#[from] database::Error),
}

impl_error_from!(users::database::UpdateUserError => UpdateUserError => {
	E::UserDoesNotExist => Self::UserDoesNotExist,
	E::Database(source) => Self::Database(source),
});
