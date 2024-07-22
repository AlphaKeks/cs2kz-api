//! Request / Response types for this service.

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};

use crate::services::auth::session::user;
use crate::util::num::ClampedU64;

/// Request payload for fetching an admin.
#[derive(Debug, Deserialize)]
pub struct FetchAdminRequest
{
	/// The admin's SteamID.
	pub user_id: SteamID,
}

/// Response payload for fetching an admin.
#[derive(Debug, Serialize)]
pub struct FetchAdminResponse
{
	/// The admin's name.
	pub name: String,

	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's permissions.
	pub permissions: user::Permissions,
}

/// Request payload for fetching many admins.
#[derive(Debug, Default, Deserialize)]
pub struct FetchAdminsRequest
{
	/// Only include admins with these permissions.
	#[serde(default)]
	pub required_permissions: user::Permissions,

	/// The maximum amount of admins to return.
	#[serde(default)]
	pub limit: ClampedU64<{ u64::MAX }>,

	/// Pagination offset.
	#[serde(default)]
	pub offset: ClampedU64,
}

/// Response payload for fetching many admins.
#[derive(Debug, Serialize)]
pub struct FetchAdminsResponse
{
	/// The admins.
	pub admins: Vec<FetchAdminResponse>,

	/// How many admins **could have been** fetched, if there was no limit.
	pub total: u64,
}

/// Request payload for updating a user's permissions.
#[derive(Debug)]
pub struct SetPermissionsRequest
{
	/// The user's SteamID.
	pub user_id: SteamID,

	/// The permissions to set for the user.
	pub permissions: user::Permissions,
}
