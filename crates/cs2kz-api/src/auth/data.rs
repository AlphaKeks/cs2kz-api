use cs2kz::SteamID;
use derive_more::Constructor;

use super::Permissions;
use crate::util::time::Timestamp;

#[derive(Debug, Clone, Constructor)]
pub struct SessionData
{
	pub(super) user_id: SteamID,
	pub(super) permissions: Permissions,
	pub(super) expires_on: Timestamp,
}

impl SessionData
{
	pub fn user_id(&self) -> SteamID
	{
		self.user_id
	}

	pub fn permissions(&self) -> Permissions
	{
		self.permissions
	}

	pub fn has_expired(&self) -> bool
	{
		self.expires_on <= Timestamp::now()
	}
}
