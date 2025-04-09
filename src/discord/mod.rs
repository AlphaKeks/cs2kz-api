//! Discord Bot
//!
//! This module contains the API's Discord Bot, responsible for assigning roles
//! to users and making announcements.

pub use self::{config::Config, token::Token, tracing_layer::Layer as TracingLayer};
use {
	crate::{
		database::{self, DatabaseError},
		event_queue::{self, Event},
		maps::{MapId, MapName},
		players::PlayerId,
		records::RecordId,
		servers::{self, ServerId},
		users::{Permissions, UserId},
	},
	futures_util::{StreamExt, TryFutureExt},
	poise::serenity_prelude::{
		self as serenity,
		ActivityData,
		CreateEmbed,
		CreateMessage,
		GatewayIntents,
		Member,
		RatelimitInfo,
		Ready,
		ResumedEvent,
	},
	std::{collections::HashSet, error::Error, pin::pin, sync::Arc},
	tokio::sync::mpsc,
	tokio_util::sync::CancellationToken,
};

mod commands;
pub mod config;
mod token;
mod tracing_layer;

type Context<'a> = poise::Context<'a, State, DiscordError>;

const GATEWAY_INTENTS: GatewayIntents = GatewayIntents::GUILD_MEMBERS;

/// The API's Discord Bot
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: crate::discord
#[derive(Debug)]
pub struct Bot
{
	/// The original sender half of the command queue
	///
	/// This is used to create weak senders for [`BotHandle`]s.
	tx: mpsc::Sender<BotMessage>,

	/// The receiver half of the command queue
	///
	/// This is used to receive messages from [`BotHandle`]s.
	rx: mpsc::Receiver<BotMessage>,

	/// The original sender half of the log queue
	///
	/// This is used to create [`TracingLayer`]s for capturing application logs.
	log_tx: mpsc::Sender<CreateEmbed>,

	/// The receiver half of the log queue
	///
	/// This is used for receiving log data from [`TracingLayer`]s so we can
	/// send log messages to Discord channels.
	log_rx: mpsc::Receiver<CreateEmbed>,

	config: Arc<Config>,
	database: database::ConnectionPool,
}

/// A handle to a [`Bot`]
///
/// This can be used to send messages to a running bot process.
#[derive(Debug, Clone)]
pub struct BotHandle
{
	tx: mpsc::WeakSender<BotMessage>,
}

#[derive(Debug, Display, Error, From)]
pub enum DiscordError
{
	/// An error from the underlying Discord library
	#[from]
	Serenity(serenity::Error),

	/// An error from communicating with the database
	#[from(DatabaseError, sqlx::Error)]
	DatabaseError(DatabaseError),
}

/// State we pass to serenity's context
#[derive(Debug)]
struct State
{
	config: Arc<Config>,
	database: database::ConnectionPool,
}

#[derive(Debug)]
enum BotMessage
{
	/// Assign the "mapper" role to a user
	AssignMapperRole
	{
		user_id: UserId
	},

	/// Remove the "mapper" role from a user
	RevokeMapperRole
	{
		user_id: UserId
	},
}

impl Bot
{
	/// Creates a new [`Bot`].
	#[instrument(skip(config, database), ret(level = "debug"), err)]
	pub fn new(config: Config, database: database::ConnectionPool) -> Result<Self, DiscordError>
	{
		let (tx, rx) = mpsc::channel(16);
		let (log_tx, log_rx) = mpsc::channel(64);
		let config = Arc::new(config);

		Ok(Self { tx, rx, log_tx, log_rx, config, database })
	}

	/// Returns a [handle] to this [`Bot`].
	///
	/// [handle]: BotHandle
	pub fn handle(&self) -> BotHandle
	{
		BotHandle { tx: self.tx.downgrade() }
	}

	/// Returns a [`tracing_subscriber::Layer`] that will send logs to `self` so
	/// they can be posted as Discord messages.
	pub fn tracing_layer(&self) -> TracingLayer
	{
		TracingLayer::new(&self.log_tx)
	}

	/// Runs the bot.
	#[instrument(skip(self, cancellation_token), err)]
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

		let mut client = serenity::Client::builder(self.config.token.as_str(), GATEWAY_INTENTS)
			.framework(framework)
			.activity(ActivityData::custom("(͡ ͡° ͜ つ ͡͡°)"))
			.await?;

		let http = Arc::clone(&client.http);

		// We need this extra scope so the `client.start()` future is dropped
		// before we access `client` again.
		// Because it is pinned locally, `drop(client_future)` would drop the
		// `Pin` rather than the future itself.
		{
			let mut client_future = pin!(client.start());
			let mut events = pin!(event_queue::subscribe());

			loop {
				select! {
					() = cancellation_token.cancelled() => {
						info!("discord bot shutting down");
						break;
					},

					client_result = &mut client_future => match client_result {
						Ok(()) => break,
						Err(err) => {
							error!(error = &err as &dyn Error, "failed to run discord bot");
							return Err(err.into());
						},
					},

					Some(event) = events.next() => match *event {
						Event::Lag { skipped } => {
							warn!(skipped, "missed events");
						},
						Event::MapCreated { id, ref name } => {
							self.on_map_created(id, name).await?;
						},
						Event::MapApproved { id } => {
							self.on_map_approved(id).await?;
						},
						Event::ServerConnected { id, ref connection_info } => {
							self.on_server_connected(id, connection_info).await?;
						},
						Event::ServerDisconnected { id } => {
							self.on_server_disconnected(id).await?;
						},
						Event::PlayerJoin { server_id, ref player } => {
							self.on_player_join(server_id, player).await?;
						},
						Event::PlayerLeave { server_id, player_id } => {
							self.on_player_leave(server_id, player_id).await?;
						},
						Event::RecordSubmitted { record_id } => {
							self.on_record_submitted(record_id).await?;
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

	#[instrument(skip(self, http), err)]
	async fn assign_mapper_role(
		&mut self,
		http: &serenity::Http,
		user_id: UserId,
	) -> Result<(), DiscordError>
	{
		let Some(mapper_role_id) = self.config.roles.mapper else {
			warn!("no mapper role configured");
			return Ok(());
		};

		let discord_user_id = {
			let mut db_conn = self.database.acquire().await?;
			sqlx::query_scalar!("SELECT discord_id FROM Users WHERE id = ?", user_id)
				.fetch_optional(db_conn.raw_mut())
				.await?
		};

		if let Some(Some(user_id)) = discord_user_id {
			if let Ok(member) = self.config.guild_id.member(http, user_id).await {
				member.add_role(http, mapper_role_id).await?;
				info!(username = member.user.name, "assigned mapper role to user");
			}
		}

		Ok(())
	}

	#[instrument(skip(self, http), err)]
	async fn revoke_mapper_role(
		&mut self,
		http: &serenity::Http,
		user_id: UserId,
	) -> Result<(), DiscordError>
	{
		let Some(mapper_role_id) = self.config.roles.mapper else {
			warn!("no mapper role configured");
			return Ok(());
		};

		let discord_user_id = {
			let mut db_conn = self.database.acquire().await?;
			sqlx::query_scalar!("SELECT discord_id FROM Users WHERE id = ?", user_id)
				.fetch_optional(db_conn.raw_mut())
				.await?
		};

		if let Some(Some(user_id)) = discord_user_id {
			if let Ok(member) = self.config.guild_id.member(http, user_id).await {
				member.remove_role(http, mapper_role_id).await?;
				info!(username = member.user.name, "revoked mapper role from user");
			}
		}

		Ok(())
	}

	#[instrument(skip(self, http, embed), err)]
	async fn send_log_message(
		&mut self,
		http: &serenity::Http,
		embed: CreateEmbed,
	) -> Result<(), DiscordError>
	{
		let message = CreateMessage::default().embed(embed);

		trace!(?message, "sending message");
		self.config.log_channel_id.send_message(http, message).await?;

		Ok(())
	}

	#[instrument(skip(self), err)]
	async fn on_map_created(&mut self, id: MapId, name: &MapName) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_map_approved(&mut self, id: MapId) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_server_connected(
		&mut self,
		id: ServerId,
		connection_info: &servers::ConnectionInfo,
	) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_server_disconnected(&mut self, id: ServerId) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_player_join(
		&mut self,
		server_id: ServerId,
		player: &servers::ConnectedPlayerInfo,
	) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_player_leave(
		&mut self,
		server_id: ServerId,
		player_id: PlayerId,
	) -> Result<(), DiscordError>
	{
		todo!()
	}

	#[instrument(skip(self), err)]
	async fn on_record_submitted(&mut self, record_id: RecordId) -> Result<(), DiscordError>
	{
		todo!()
	}
}

impl BotHandle
{
	/// Creates a dangling handle.
	///
	/// Calls to this handle will always fail.
	pub fn dangling() -> Self
	{
		let (tx, _) = mpsc::channel(1);
		Self { tx: tx.downgrade() }
	}

	/// Tells the bot to assign the "mapper" role to a user.
	#[instrument(skip(self), ret(level = "debug"))]
	pub async fn assign_mapper_role(&self, user_id: UserId) -> bool
	{
		let Some(tx) = self.tx.upgrade() else {
			return false;
		};

		tx.send(BotMessage::AssignMapperRole { user_id }).await.is_ok()
	}

	/// Tells the bot to revoke the "mapper" role from a user.
	#[instrument(skip(self), ret(level = "debug"))]
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

#[instrument(skip_all, ret(level = "debug"), err)]
async fn framework_setup(
	cx: &serenity::Context,
	_ready: &serenity::Ready,
	framework: &poise::Framework<State, DiscordError>,
	config: Arc<Config>,
	database: database::ConnectionPool,
) -> Result<State, DiscordError>
{
	poise::builtins::register_in_guild(&cx.http, &framework.options().commands, config.guild_id)
		.await?;

	Ok(State { config, database })
}

#[instrument(level = "error", skip_all)]
async fn on_error(error: poise::FrameworkError<'_, State, DiscordError>)
{
	// `error` cannot be recorded as `&(dyn Error + 'static)` because the
	// lifetime parameter in `FrameworkError` is unspecified and therefore it is
	// not `'static`.
	error!(%error);
}

#[instrument(level = "trace", skip_all)]
async fn pre_command(cx: Context<'_>)
{
	trace!(command = cx.invoked_command_name(), "executing command");
}

#[instrument(level = "trace", skip_all)]
async fn post_command(cx: Context<'_>)
{
	trace!(command = cx.invoked_command_name(), "executed command");
}

#[instrument(skip_all, err)]
async fn on_event(
	client_cx: &serenity::Context,
	event: &serenity::FullEvent,
	_framework_cx: poise::FrameworkContext<'_, State, DiscordError>,
	state: &State,
) -> Result<(), DiscordError>
{
	debug!(event = event.snake_case_name(), "received event");

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

#[instrument(skip(client_cx, state), err)]
async fn on_guild_member_addition(
	client_cx: &serenity::Context,
	state: &State,
	member: &Member,
) -> Result<(), DiscordError>
{
	if member.guild_id != state.config.guild_id {
		trace!("ignoring irrelevant guild");
		return Ok(());
	}

	debug!("new member joined, assigning roles");

	let mut db_conn = state.database.acquire().await?;

	let user_info = sqlx::query!(
		"SELECT id, permissions AS `permissions: Permissions`
		 FROM Users
		 WHERE discord_id = ?",
		member.user.id.get(),
	)
	.fetch_optional(db_conn.raw_mut())
	.await?;

	let owns_servers = if let Some(user_id) = user_info.as_ref().map(|info| info.id) {
		sqlx::query_scalar!("SELECT COUNT(*) FROM Servers WHERE owner_id = ?", user_id)
			.fetch_one(db_conn.raw_mut())
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
		.chain(if owns_servers { state.config.roles.server_owner } else { None })
		.inspect(|role_id| trace!(id = %role_id, "assigning role"))
		.collect::<Vec<_>>();

	if roles_to_add.is_empty() {
		debug!("no roles to assign");
	} else {
		member.add_roles(client_cx, &roles_to_add[..]).await?;
		debug!("assigned roles successfully");
	}

	Ok(())
}

#[instrument(err)]
async fn on_ready(ready: &Ready) -> Result<(), DiscordError>
{
	info!("discord bot is online");

	Ok(())
}

#[instrument(err)]
async fn on_resume(event: &ResumedEvent) -> Result<(), DiscordError>
{
	warn!("discord bot was disconnected but is back online");

	Ok(())
}

#[instrument(err)]
async fn on_ratelimit(data: &RatelimitInfo) -> Result<(), DiscordError>
{
	warn!("getting rate limited");

	Ok(())
}
