use std::fmt::Write;

use futures_util::TryFutureExt;
use poise::{
	CreateReply,
	serenity_prelude::{CreateEmbed, User},
};

use super::{Context, DiscordError, config::Roles};
use crate::users::{Permission, Permissions};

/// Synchronizes a user's roles with their API permissions.
#[tracing::instrument(skip(cx), err)]
#[poise::command(
	slash_command,
	context_menu_command = "Sync Roles with API permissions",
	required_permissions = "MANAGE_ROLES",
	required_bot_permissions = "MANAGE_ROLES",
	guild_only
)]
pub(super) async fn sync_roles(cx: Context<'_>, user: User) -> Result<(), DiscordError>
{
	cx.defer_ephemeral().await?;

	let member = cx.data().config.guild_id.member(cx.http(), user.id).await?;
	let mut conn = cx.data().database.acquire_connection().await?;

	let Some(user_info) = sqlx::query!(
		"SELECT id, permissions AS `permissions: Permissions`
		 FROM Users
		 WHERE discord_id = ?",
		user.id.get(),
	)
	.fetch_optional(conn.as_raw())
	.await?
	else {
		let reply = CreateReply::default()
			.content("User is not registered with the API.")
			.ephemeral(true)
			.reply(true);

		cx.send(reply).await?;
		return Ok(());
	};

	let mut roles_to_keep = Vec::with_capacity(member.roles.len());
	let mut roles_to_remove = Vec::with_capacity(member.roles.len());

	for &role_id in &member.roles {
		let Roles { mapper, server_owner } = cx.data().config.roles;

		if mapper == Some(role_id) && !user_info.permissions.contains(&Permission::CreateMaps) {
			roles_to_remove.push(role_id);
			continue;
		}

		if server_owner == Some(role_id) {
			let owns_servers = sqlx::query_scalar!(
				"SELECT COUNT(*)
				 FROM Servers
				 WHERE owner_id = ?",
				user_info.id,
			)
			.fetch_one(conn.as_raw())
			.map_ok(|server_count| server_count > 0)
			.await?;

			if !owns_servers {
				roles_to_remove.push(role_id);
				continue;
			}
		}

		roles_to_keep.push(role_id);
	}

	member.add_roles(cx.http(), &roles_to_keep).await?;
	member.remove_roles(cx.http(), &roles_to_remove).await?;

	let reply = CreateReply::default().ephemeral(true).reply(true).embed({
		let mut embed = CreateEmbed::default().title("Synced Roles");

		let added_roles = roles_to_keep.iter().fold(String::new(), |mut text, role_id| {
			if roles_to_keep.first() != Some(role_id) {
				let _ = write!(text, ", ");
			}

			let _ = write!(text, "<@&{role_id}>");
			text
		});

		if !added_roles.is_empty() {
			embed = embed.field("Added", added_roles, false);
		}

		let removed_roles = roles_to_remove.iter().fold(String::new(), |mut text, role_id| {
			if roles_to_remove.first() != Some(role_id) {
				let _ = write!(text, ", ");
			}

			let _ = write!(text, "<@&{role_id}>");
			text
		});

		if !removed_roles.is_empty() {
			embed = embed.field("Removed", removed_roles, false);
		}

		embed
	});

	cx.send(reply).await?;

	Ok(())
}
