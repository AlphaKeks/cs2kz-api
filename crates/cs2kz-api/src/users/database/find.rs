use futures::stream::{StreamExt, TryStreamExt};

use crate::database::{self, RowStream};
use crate::email::EmailAddress;
use crate::time::Timestamp;
use crate::users::permissions::Permissions;
use crate::users::UserID;

/// A row in the `Users` table.
#[derive(Debug)]
pub struct User {
	pub id: UserID,
	pub permissions: Permissions,
	pub email: Option<EmailAddress>,
	pub created_at: Timestamp,
	pub last_seen_at: Timestamp,
}

pub struct FindUsers {
	/// Only include users with at least these permissions.
	pub permissions: Permissions,

	/// Also include users who have no permissions at all.
	pub allow_empty_permissions: bool,
}

/// Returns a [`Stream`] of [`User`]s that match the given parameters.
#[instrument(level = "debug", skip(conn))]
pub fn find(
	conn: &mut database::Connection,
	FindUsers {
		permissions,
		allow_empty_permissions,
	}: FindUsers,
) -> impl RowStream<'_, User> {
	sqlx::query!(
		"SELECT
		   id `id: UserID`,
		   permissions `permissions: Permissions`,
		   email,
		   created_at `created_at: Timestamp`,
		   last_seen_at `last_seen_at: Timestamp`
		 FROM Users
		 WHERE ?
		 OR (permissions & ?) = ?",
		allow_empty_permissions,
		permissions,
		permissions,
	)
	.fetch(conn)
	.map(|row| {
		let row = row?;

		Ok(User {
			id: row.id,
			permissions: row.permissions,
			email: if row.email.is_empty() {
				None
			} else {
				Some(row.email.parse::<EmailAddress>().map_err(|err| {
					sqlx::Error::ColumnDecode {
						index: String::from("email"),
						source: err.into(),
					}
				})?)
			},
			created_at: row.created_at,
			last_seen_at: row.last_seen_at,
		})
	})
	.inspect_err(|error| error!(%error, "failed to decode row"))
}

/// Finds a specific user by their ID.
#[instrument(
	level = "debug",
	skip(conn),
	ret(level = "debug"),
	err(level = "debug")
)]
pub async fn find_by_id(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<Option<User>> {
	let Some(row) = sqlx::query!(
		"SELECT
		   id `id: UserID`,
		   permissions `permissions: Permissions`,
		   email,
		   created_at `created_at: Timestamp`,
		   last_seen_at `last_seen_at: Timestamp`
		 FROM Users
		 WHERE id = ?",
		user_id,
	)
	.fetch_optional(conn)
	.await?
	else {
		return Ok(None);
	};

	Ok(Some(User {
		id: row.id,
		permissions: row.permissions,
		email: if row.email.is_empty() {
			None
		} else {
			row.email.parse::<EmailAddress>().map(Some).map_err(|err| {
				sqlx::Error::ColumnDecode {
					index: String::from("email"),
					source: err.into(),
				}
			})?
		},
		created_at: row.created_at,
		last_seen_at: row.last_seen_at,
	}))
}
