//! API users.
//!
//! A "user" is someone who interacts with the API through a web interface, such as the
//! [dashboard]. This is in contrast to a "player", which is someone playing on a CS2 server that
//! is hosting the [cs2kz-metamod plugin]. While users and players share their IDs (SteamID), they
//! are considered separate entites within the API. For example, users have permissions, while
//! players do not.
//!
//! [dashboard]: https://github.com/KZGlobalTeam/cs2kz-api-dashboard
//! [cs2kz-metamod plugin]: https://github.com/KZGlobalTeam/cs2kz-metamod

use cs2kz::SteamID;
use futures::stream::TryStreamExt;

use self::permissions::Permissions;
use crate::database::DatabaseError;
use crate::email::EmailAddress;
use crate::time::Timestamp;

pub mod permissions;

pub(crate) mod http;
pub(crate) mod sessions;

mod database;

#[derive(
	Debug, Clone, Copy, Deref, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[debug("{}", _0.as_u64())]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(value_type = u64)]
pub struct UserID(#[serde(serialize_with = "SteamID::serialize_u64_stringified")] SteamID);

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
#[schema(example = json!({
  "id": "76561198282622073",
  "permissions": ["servers", "admin"],
  "created_at": "2024-10-20T01:05:21Z"
}))]
pub struct User {
	pub id: UserID,
	pub permissions: Permissions,
	pub created_at: Timestamp,
}

impl From<self::database::User> for User {
	fn from(user: self::database::User) -> Self {
		Self {
			id: user.id,
			permissions: user.permissions,
			created_at: user.created_at,
		}
	}
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct GetUsersParams {
	/// Only return users with at least these permissions.
	#[serde(default = "Permissions::all")]
	pub permissions: Permissions,
}

#[derive(Debug)]
pub struct UserUpdate {
	pub user_id: UserID,
	pub permissions: Option<Permissions>,
	pub email: Option<UpdateEmailAddress>,
	pub mark_as_seen: bool,
}

#[derive(Debug)]
pub enum UpdateEmailAddress {
	Clear,
	Set(EmailAddress),
}

#[derive(Debug, Error)]
pub enum UpdateUserError {
	#[error(transparent)]
	Database(#[from] DatabaseError),
}

#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
pub async fn get(
	pool: &crate::database::ConnectionPool,
	user_id: UserID,
) -> crate::database::Result<Option<User>> {
	let mut conn = pool.get_connection().await?;
	let maybe_user = self::database::find_by_id(&mut conn, user_id)
		.await?
		.map(User::from);

	Ok(maybe_user)
}

#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
pub async fn get_many(
	pool: &crate::database::ConnectionPool,
	GetUsersParams { permissions }: GetUsersParams,
) -> crate::database::Result<Vec<User>> {
	if permissions.is_empty() {
		return Ok(Vec::new());
	}

	let mut conn = pool.get_connection().await?;
	let users = self::database::find(&mut conn, self::database::FindUsers {
		permissions,
		allow_empty_permissions: false,
	})
	.map_ok(User::from)
	.try_collect::<Vec<_>>()
	.await?;

	Ok(users)
}

#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
pub async fn update(
	pool: &crate::database::ConnectionPool,
	UserUpdate {
		user_id,
		permissions,
		email,
		mark_as_seen,
	}: UserUpdate,
) -> crate::database::Result<bool> {
	let mut txn = pool.begin_transaction().await?;
	let updated = self::database::update(&mut txn, self::database::UserUpdate {
		user_id,
		permissions,
		email: email.as_ref().map(|update| match update {
			UpdateEmailAddress::Clear => self::database::UpdateEmailAddress::Clear,
			UpdateEmailAddress::Set(email) => self::database::UpdateEmailAddress::Set(email),
		}),
	})
	.await?;

	if updated {
		if mark_as_seen {
			self::database::mark_as_seen(&mut txn, user_id).await?;
		}

		txn.commit().await.map_err(DatabaseError::from)?;
	}

	Ok(updated)
}
