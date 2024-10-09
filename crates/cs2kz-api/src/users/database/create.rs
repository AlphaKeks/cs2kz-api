use cs2kz::SteamID;

use crate::database::{self, ErrorExt};
use crate::users::UserID;

#[instrument(skip(conn), err(level = "debug"))]
pub async fn create(
	conn: &mut database::Connection,
	user_id: UserID,
) -> Result<(), CreateUserError> {
	sqlx::query! {
		"INSERT INTO Users (id)
		 VALUES (?)",
		user_id,
	}
	.execute(conn)
	.await
	.map_err(|error| {
		if error.is_unique_violation() {
			CreateUserError::DuplicateUserID
		} else {
			CreateUserError::Database(error)
		}
	})?;

	Ok(())
}

#[derive(Debug, Error)]
pub enum CreateUserError {
	#[error("duplicate user ID")]
	DuplicateUserID,

	#[error(transparent)]
	Database(#[from] database::Error),
}

#[cfg(test)]
mod tests {
	use rand::Rng;

	use super::*;
	use crate::testing;

	#[sqlx::test(migrator = "crate::database::MIGRATIONS")]
	async fn create_works(pool: database::ConnectionPool) -> testing::Result {
		let mut conn = pool.acquire().await?;
		let user_id = rand::random::<SteamID>().into();
		let result = super::create(&mut conn, user_id).await;

		assert_matches!(result, Ok(()));

		Ok(())
	}

	#[sqlx::test(migrator = "crate::database::MIGRATIONS")]
	async fn create_rejects_duplicate(pool: database::ConnectionPool) -> testing::Result {
		let mut conn = pool.acquire().await?;
		let user_id = rand::random::<SteamID>().into();

		for i in 0..2 {
			let result = super::create(&mut conn, user_id).await;

			if i == 0 {
				assert_matches!(result, Ok(()));
			} else {
				assert_matches!(result, Err(CreateUserError::DuplicateUserID));
			}
		}

		Ok(())
	}
}
