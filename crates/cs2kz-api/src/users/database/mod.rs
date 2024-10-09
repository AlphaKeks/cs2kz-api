//! Functions to interact with the `Users` table.

use time::OffsetDateTime;

use crate::users::email::Email;
use crate::users::permissions::Permissions;
use crate::users::UserID;

mod create;
pub use create::{create, CreateUserError};

mod get;
pub use get::{get_admins, get_by_id};

mod update;
pub use update::{update, UpdateUserError, UserUpdate};

#[derive(Debug, Clone, PartialEq)]
pub struct User {
	pub id: UserID,
	pub name: Option<String>,
	pub permissions: Permissions,
	pub email: Option<Email>,
	pub created_at: OffsetDateTime,
	pub last_seen_at: OffsetDateTime,
}
