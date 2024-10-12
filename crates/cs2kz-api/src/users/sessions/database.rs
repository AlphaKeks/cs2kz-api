//! Functions to interact with the `UserSessions` table.

use time::OffsetDateTime;

use crate::users::permissions::Permissions;
use crate::users::sessions::SessionID;
use crate::users::UserID;

mod create;
pub use create::{create, CreateSessionError};

mod get;
pub use get::get_by_id;

mod update;
pub use update::{extend_session, invalidate_session, invalidate_sessions};

#[derive(Debug, Clone, PartialEq)]
pub struct Session {
	pub id: SessionID,
	pub user_id: UserID,
	pub user_permissions: Permissions,
	pub created_at: OffsetDateTime,
	pub expires_at: OffsetDateTime,
}
