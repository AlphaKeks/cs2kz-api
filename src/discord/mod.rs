mod commands;
pub mod config;
mod token;
mod tracing_layer;

use std::{collections::HashSet, error::Error, pin::pin, sync::Arc};

use futures_util::TryFutureExt;
use poise::serenity_prelude::{
	self as serenity,
	ActivityData,
	CreateEmbed,
	CreateMessage,
	GatewayIntents,
	Member,
	RatelimitInfo,
	Ready,
	ResumedEvent,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub use self::{config::Config, token::Token, tracing_layer::Layer as TracingLayer};
use crate::{
	database::{Database, DatabaseError},
	users::{Permissions, UserId},
};

type Context<'a> = poise::Context<'a, State, DiscordError>;

#[derive(Debug)]
pub struct Bot
{
	tx: mpsc::Sender<BotMessage>,
	rx: mpsc::Receiver<BotMessage>,

	log_tx: mpsc::Sender<CreateEmbed>,
	log_rx: mpsc::Receiver<CreateEmbed>,

	config: Arc<Config>,
	database: Database,
}

#[derive(Debug, Clone)]
pub struct BotHandle
{
	tx: mpsc::WeakSender<BotMessage>,
}

#[derive(Debug, Display, Error, From)]
pub enum DiscordError
{
	#[from]
	Serenity(serenity::Error),

	#[from(DatabaseError, sqlx::Error)]
	Database(DatabaseError),
}

#[derive(Debug)]
struct State
{
	config: Arc<Config>,
	database: Database,
}

#[derive(Debug)]
enum BotMessage
{
	AssignMapperRole
	{
		user_id: UserId
	},

	RevokeMapperRole
	{
		user_id: UserId
	},
}

impl Bot
{
	#[tracing::instrument(skip(database), ret(level = "debug"), err)]
	pub async fn new(config: Config, database: Database) -> Result<Self, DiscordError>
	{
		let (tx, rx) = mpsc::channel(16);
		let (log_tx, log_rx) = mpsc::channel(64);
		let config = Arc::new(config);

		Ok(Self { tx, rx, log_tx, log_rx, config, database })
	}

	pub fn handle(&self) -> BotHandle
	{
		BotHandle { tx: self.tx.downgrade() }
	}

	pub fn tracing_layer(&self) -> TracingLayer
	{
		TracingLayer::new(&self.log_tx)
	}

	#[tracing::instrument(skip(self, cancellation_token), err)]
	pub async fn run(mut self, cancellation_token: CancellationToken) -> Result<(), DiscordError>
	{
		let framework = poise::Framework::builder()
			.options(framework_options(self.config.owners.iter().copied()))
			.setup({
				let config = Arc::clone(&self.config);
				let database = self.database.clone();
				|cx, ready, framework| {
					Box::pin(framework_setup(cx, ready, framework, config, database))
				}
			})
			.build();

		let mut client = serenity::Client::builder(self.config.token.as_str(), gateway_intents())
			.framework(framework)
			.activity(ActivityData::custom("(͡ ͡° ͜ つ ͡͡°)"))
			.await?;

		let http = Arc::clone(&client.http);

		{
			let mut client_future = pin!(client.start());
			loop {
				select! {
					() = cancellation_token.cancelled() => {
						tracing::info!("discord bot shutting down");
						break;
					},

					client_result = &mut client_future => match client_result {
						Ok(()) => break,
						Err(err) => {
							tracing::error!(error = &err as &dyn Error, "failed to run discord bot");
							return Err(err.into());
						},
					},

					Some(message) = self.rx.recv() => match message {
						BotMessage::AssignMapperRole { user_id } => {
							self.assign_mapper_role(&http, user_id).await?;
						},
						BotMessage::RevokeMapperRole { user_id } => {
							self.revoke_mapper_role(&http, user_id).await?;
						},
					},

					Some(embed) = self.log_rx.recv() => {
						self.send_log_message(&http, embed).await?;
					},
				};
			}
		}

		client.shard_manager.shutdown_all().await;

		Ok(())
	}

	#[tracing::instrument(skip(self, http), err)]
	async fn assign_mapper_role(
		&mut self,
		http: &serenity::Http,
		user_id: UserId,
	) -> Result<(), DiscordError>
	{
		let Some(mapper_role_id) = self.config.roles.mapper else {
			tracing::warn!("no mapper role configured");
			return Ok(());
		};

		let discord_user_id = {
			let mut conn = self.database.acquire_connection().await?;
			sqlx::query_scalar!("SELECT discord_id FROM Users WHERE id = ?", user_id)
				.fetch_optional(conn.as_raw())
				.await?
		};

		if let Some(Some(user_id)) = discord_user_id {
			if let Ok(member) = self.config.guild_id.member(http, user_id).await {
				member.add_role(http, mapper_role_id).await?;
				tracing::info!(username = member.user.name, "assigned mapper role to user");
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip(self, http), err)]
	async fn revoke_mapper_role(
		&mut self,
		http: &serenity::Http,
		user_id: UserId,
	) -> Result<(), DiscordError>
	{
		let Some(mapper_role_id) = self.config.roles.mapper else {
			tracing::warn!("no mapper role configured");
			return Ok(());
		};

		let discord_user_id = {
			let mut conn = self.database.acquire_connection().await?;
			sqlx::query_scalar!("SELECT discord_id FROM Users WHERE id = ?", user_id)
				.fetch_optional(conn.as_raw())
				.await?
		};

		if let Some(Some(user_id)) = discord_user_id {
			if let Ok(member) = self.config.guild_id.member(http, user_id).await {
				member.remove_role(http, mapper_role_id).await?;
				tracing::info!(username = member.user.name, "revoked mapper role from user");
			}
		}

		Ok(())
	}

	#[tracing::instrument(skip(self, http), err)]
	async fn send_log_message(
		&mut self,
		http: &serenity::Http,
		embed: CreateEmbed,
	) -> Result<(), DiscordError>
	{
		self.config
			.log_channel_id
			.send_message(http, CreateMessage::default().embed(embed))
			.await?;

		Ok(())
	}
}

impl BotHandle
{
	/// Creates a dangling handle.
	///
	/// Calls to this handle will always return a "bot unavailable" error.
	pub fn dangling() -> Self
	{
		let (tx, _) = mpsc::channel(1);
		Self { tx: tx.downgrade() }
	}

	/// Tells the bot to assign the mapper role to a user.
	#[tracing::instrument(skip(self), ret(level = "debug"))]
	pub async fn assign_mapper_role(&self, user_id: UserId) -> bool
	{
		let Some(tx) = self.tx.upgrade() else {
			return false;
		};

		tx.send(BotMessage::AssignMapperRole { user_id }).await.is_ok()
	}

	/// Tells the bot to revoke the mapper role from a user.
	#[tracing::instrument(skip(self), ret(level = "debug"))]
	pub async fn revoke_mapper_role(&self, user_id: UserId) -> bool
	{
		let Some(tx) = self.tx.upgrade() else {
			return false;
		};

		tx.send(BotMessage::RevokeMapperRole { user_id }).await.is_ok()
	}
}

fn framework_options(
	owner_ids: impl IntoIterator<Item = serenity::UserId>,
) -> poise::FrameworkOptions<State, DiscordError>
{
	poise::FrameworkOptions {
		commands: vec![commands::sync_roles()],
		on_error: |error| Box::pin(on_error(error)),
		pre_command: |cx| Box::pin(pre_command(cx)),
		post_command: |cx| Box::pin(post_command(cx)),
		event_handler: |client_cx, event, framework_cx, state| {
			Box::pin(on_event(client_cx, event, framework_cx, state))
		},
		owners: HashSet::from_iter(owner_ids),
		..Default::default()
	}
}

fn gateway_intents() -> GatewayIntents
{
	GatewayIntents::GUILD_MEMBERS
}

#[tracing::instrument(skip_all, ret(level = "debug"), err)]
async fn framework_setup(
	cx: &serenity::Context,
	_ready: &serenity::Ready,
	framework: &poise::Framework<State, DiscordError>,
	config: Arc<Config>,
	database: Database,
) -> Result<State, DiscordError>
{
	poise::builtins::register_in_guild(&cx.http, &framework.options().commands, config.guild_id)
		.await?;

	Ok(State { config, database })
}

#[tracing::instrument(level = "error", skip_all)]
async fn on_error(error: poise::FrameworkError<'_, State, DiscordError>)
{
	tracing::error!(%error);
}

#[tracing::instrument(level = "trace", skip_all)]
async fn pre_command(cx: Context<'_>)
{
	tracing::trace!(command = cx.invoked_command_name(), "executing command");
}

#[tracing::instrument(level = "trace", skip_all)]
async fn post_command(cx: Context<'_>)
{
	tracing::trace!(command = cx.invoked_command_name(), "executed command");
}

#[tracing::instrument(skip_all, err)]
async fn on_event(
	client_cx: &serenity::Context,
	event: &serenity::FullEvent,
	_framework_cx: poise::FrameworkContext<'_, State, DiscordError>,
	state: &State,
) -> Result<(), DiscordError>
{
	tracing::debug!(event = event.snake_case_name(), "received event");

	#[allow(non_exhaustive_omitted_patterns, clippy::wildcard_enum_match_arm)]
	match event {
		serenity::FullEvent::GuildMemberAddition { new_member } => {
			on_guild_member_addition(client_cx, state, new_member).await
		},
		serenity::FullEvent::Ready { data_about_bot } => on_ready(data_about_bot).await,
		serenity::FullEvent::Resume { event } => on_resume(event).await,
		serenity::FullEvent::Ratelimit { data } => on_ratelimit(data).await,
		_ => Ok(()),
	}
}

#[tracing::instrument(skip(client_cx, state), err)]
async fn on_guild_member_addition(
	client_cx: &serenity::Context,
	state: &State,
	member: &Member,
) -> Result<(), DiscordError>
{
	if member.guild_id != state.config.guild_id {
		tracing::trace!("ignoring irrelevant guild");
		return Ok(());
	}

	tracing::debug!("new member joined, assigning roles");

	let mut conn = state.database.acquire_connection().await?;

	let user_info = sqlx::query!(
		"SELECT id, permissions AS `permissions: Permissions`
		 FROM Users
		 WHERE discord_id = ?",
		member.user.id.get(),
	)
	.fetch_optional(conn.as_raw())
	.await?;

	let owns_servers = if let Some(user_id) = user_info.as_ref().map(|info| info.id) {
		sqlx::query_scalar!("SELECT COUNT(*) FROM Servers WHERE owner_id = ?", user_id)
			.fetch_one(conn.as_raw())
			.map_ok(|server_count| server_count > 0)
			.await?
	} else {
		false
	};

	let roles_to_add = user_info
		.as_ref()
		.map(|info| info.permissions)
		.into_iter()
		.flatten()
		.filter_map(|permission| state.config.roles.id_for_permission(permission))
		.chain(if owns_servers {
			state.config.roles.server_owner
		} else {
			None
		})
		.inspect(|role_id| tracing::trace!(id = %role_id, "assigning role"))
		.collect::<Vec<_>>();

	if roles_to_add.is_empty() {
		tracing::debug!("no roles to assign");
	} else {
		member.add_roles(client_cx, &roles_to_add[..]).await?;
		tracing::debug!("assigned roles successfully");
	}

	Ok(())
}

#[tracing::instrument(err)]
async fn on_ready(ready: &Ready) -> Result<(), DiscordError>
{
	tracing::info!("discord bot is online");

	Ok(())
}

#[tracing::instrument(err)]
async fn on_resume(event: &ResumedEvent) -> Result<(), DiscordError>
{
	tracing::warn!("discord bot was disconnected but is back online");

	Ok(())
}

#[tracing::instrument(err)]
async fn on_ratelimit(data: &RatelimitInfo) -> Result<(), DiscordError>
{
	tracing::warn!("getting rate limited");

	Ok(())
}
