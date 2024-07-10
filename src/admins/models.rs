//! Types for modeling KZ admins.

use cs2kz::SteamID;
use derive_more::Debug;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::authorization::Permissions;
use crate::openapi::parameters::{Limit, Offset};

/// A KZ admin.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Admin
{
	/// The admin's name.
	pub name: String,

	/// The admin's SteamID.
	pub steam_id: SteamID,

	/// The admin's permissions.
	#[debug("{permissions:?} ({permissions})")]
	#[schema(value_type = Vec<String>, example = json!(["bans", "servers"]))]
	pub permissions: Permissions,
}

/// Query parameters for fetching admins.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct FetchAdminsRequest
{
	/// Filter by permissions.
	#[param(value_type = Vec<String>)]
	#[serde(default)]
	pub permissions: Permissions,

	/// Maximum number of results to return.
	#[serde(default)]
	pub limit: Limit,

	/// Pagination offset.
	#[serde(default)]
	pub offset: Offset,
}

/// Request payload for updating an admin.
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct AdminUpdate
{
	/// New set of permissions for the admin.
	#[debug("{permissions}")]
	#[schema(value_type = Vec<String>, example = json!(["bans", "servers"]))]
	pub permissions: Permissions,
}
