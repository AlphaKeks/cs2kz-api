//! Everything related to "users".
//!
//! "users" are different from ["players"], in that they're kept track of separately. While they
//! share IDs (their SteamID), "users" can login/logout, have permissions, and interact with the
//! API via the [dashboard]. "players" play on servers.
//!
//! ["players"]: crate::players
//! [dashboard]: https://github.com/KZGlobalTeam/cs2kz-api-dashboard

pub mod permissions;
pub mod email;
pub mod sessions;

mod database;
pub use database::{get_admins, get_by_id, User, UserUpdate};

mod register;
pub use register::{register, RegisterUserError};

mod update;
pub use update::{update, UpdateUserError};

#[derive(
	Debug, Clone, Copy, PartialEq, From, Deref, serde::Serialize, serde::Deserialize, sqlx::Type,
)]
#[debug("{}", _0.as_u64())]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct UserID(cs2kz::SteamID);
