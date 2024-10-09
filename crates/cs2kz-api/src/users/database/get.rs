use cs2kz::SteamID;
use futures::stream::{Stream, TryStreamExt};

use super::User;
use crate::database;
use crate::users::permissions::Permissions;
use crate::users::UserID;

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	user_id: UserID,
) -> database::Result<Option<User>> {
	let maybe_user = macros::query!("WHERE u.id = ?", user_id)
		.fetch_optional(conn)
		.await?
		.map(|user| macros::map_row!(user));

	Ok(maybe_user)
}

/// Gets "admins" (users with permissions).
///
/// The `permissions` parameter can be used to ensure the returned users include **at least**
/// specific permissions. If any permissions are fine, pass `Permissions::default()`.
#[instrument(skip(conn))]
pub fn get_admins(
	conn: &mut database::Connection,
	permissions: Permissions,
) -> impl Stream<Item = database::Result<User>> + Unpin + Send + '_ {
	macros::query!("WHERE (u.permissions & ?) != 0", permissions)
		.fetch(conn)
		.map_ok(|user| macros::map_row!(user))
}

mod macros {
	macro_rules! query {
		($extra_query:literal, $($args:tt)*) => {
			sqlx::query! {
				"SELECT
				   u.id `id: cs2kz::SteamID`,
				   p.name,
				   u.permissions `permissions: crate::users::permissions::Permissions`,
				   u.email,
				   u.created_at,
				   u.last_seen_at
				 FROM Users u
				 LEFT JOIN Players p ON p.id = u.id "
				+ $extra_query
				+ " ORDER BY u.created_at ASC ",
				$($args)*
			}
		};
	}

	macro_rules! map_row {
		($row:ident) => {
			$crate::users::database::User {
				id: $row.id.into(),
				name: $row.name,
				permissions: $row.permissions,
				email: if $row.email.is_empty() {
					None
				} else {
					Some(
						$row.email
							.parse()
							.expect("emails in the database should be valid"),
					)
				},
				created_at: $row.created_at,
				last_seen_at: $row.last_seen_at,
			}
		};
	}

	pub(super) use {map_row, query};
}

#[cfg(test)]
mod tests {
	use futures::stream::{self, StreamExt};

	use super::*;
	use crate::testing;
	use crate::users::permissions::{Permission, Permissions};

	const STEAM_IDS: &[u64] = &[
		76561198282622073_u64,
		76561198118681904_u64,
		76561198264939817_u64,
		76561198165203332_u64,
	];

	#[sqlx::test(migrator = "crate::database::MIGRATIONS", fixtures("users"))]
	async fn get_by_id_works(pool: database::ConnectionPool) -> testing::Result {
		let mut conn = pool.acquire().await?;

		for &steam_id in STEAM_IDS {
			let user_id = SteamID::from_u64(steam_id).map(UserID::from)?;
			let result = super::get_by_id(&mut conn, user_id).await;

			assert_matches!(result, Ok(Some(_)));
		}

		let user_id = rand::random::<SteamID>().into();
		let result = super::get_by_id(&mut conn, user_id).await;

		assert_matches!(result, Ok(None), "you should gamble professionally");

		Ok(())
	}

	#[sqlx::test(migrator = "crate::database::MIGRATIONS", fixtures("users"))]
	async fn get_admins_works(pool: database::ConnectionPool) -> testing::Result {
		let mut conn = pool.acquire().await?;
		let mut stream = super::get_admins(&mut conn, Permissions::from(Permission::Admin))
			.map_ok(|user| **user.id)
			.zip(stream::iter([STEAM_IDS[0], STEAM_IDS[3]]));

		while let Some((actual, expected)) = stream.next().await {
			assert_eq!(actual?, expected);
		}

		assert_matches!(stream.next().await, None);

		Ok(())
	}
}
