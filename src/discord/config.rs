use {
	super::Token,
	crate::users::Permission,
	poise::serenity_prelude::{self as serenity, ChannelId, GuildId, RoleId},
	serde::Deserialize,
	std::collections::HashSet,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct Config
{
	/// The bot token to be used for authentication
	///
	/// Get your token here: `https://discord.com/developers/applications/<application_id>/bot`
	pub token: Token,

	/// The ID of the primary guild
	pub guild_id: GuildId,

	/// The ID of the channel logs should be sent to
	pub log_channel_id: ChannelId,

	/// A list of users who control this bot
	#[serde(default)]
	pub owners: HashSet<serenity::UserId>,

	/// Mappings from roles the API knows about to role IDs
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
	/// Returns the [`RoleId`] that corresponds to the given `permission`.
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
