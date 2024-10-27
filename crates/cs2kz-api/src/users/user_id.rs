use std::fmt;

use cs2kz::SteamID;

/// An ID uniquely identifying a user.
#[derive(
	Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, sqlx::Type, utoipa::ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(value_type = u64)]
pub struct UserID(SteamID);

impl fmt::Debug for UserID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt.debug_tuple("UserID").field(&*self.0).finish()
	}
}
