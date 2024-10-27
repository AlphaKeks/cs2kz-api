//! API users.

use futures_util::TryStreamExt;

use crate::database::{self, RowStream};

mod user_id;
pub use user_id::UserID;

mod permissions;
pub use permissions::{Permission, Permissions, PermissionsIter};

pub mod sessions;

/// An API user.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct User {
	/// The user's ID.
	pub id: UserID,

	/// The users permissions.
	pub permissions: Permissions,
}

/// Returns the user with the given ID.
#[instrument(level = "debug", skip(conn))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<Option<User>> {
	sqlx::query_as!(
		User,
		"SELECT
		   id `id: UserID`,
		   permissions `permissions: Permissions`
		 FROM Users
		 WHERE id = ?",
		user_id,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Returns a stream of users with permissions.
#[instrument(level = "debug", skip(conn))]
pub fn get_with_permissions(conn: &mut database::Connection) -> impl RowStream<'_, User> {
	sqlx::query_as!(
		User,
		"SELECT
		   id `id: UserID`,
		   permissions `permissions: Permissions`
		 FROM Users
		 WHERE permissions > 0",
	)
	.fetch(conn.as_mut())
	.map_err(Into::into)
}
