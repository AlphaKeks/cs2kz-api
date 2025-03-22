use std::collections::HashSet;

use poise::serenity_prelude::{self as serenity, GuildId, RoleId};
use serde::Deserialize;

use super::Token;
use crate::users::Permission;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config
{
	pub token: Token,
	pub guild_id: GuildId,

	#[serde(default)]
	pub owners: HashSet<serenity::UserId>,

	#[serde(default)]
	pub roles: Roles,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub struct Roles
{
	pub mapper: Option<RoleId>,
	pub server_owner: Option<RoleId>,
}

impl Roles
{
	pub fn id_for_permission(&self, permission: Permission) -> Option<RoleId>
	{
		match permission {
			Permission::CreateMaps => self.mapper,
			Permission::UpdateMaps
			| Permission::ModifyServerMetadata
			| Permission::ModifyServerBudgets
			| Permission::ResetServerAccessKeys
			| Permission::DeleteServerAccessKeys
			| Permission::CreateBans
			| Permission::UpdateBans
			| Permission::RevertBans
			| Permission::GrantCreateMaps
			| Permission::ModifyUserPermissions => None,
		}
	}
}
