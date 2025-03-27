mod handshake;
mod message;

use std::{
	collections::btree_map::{self, BTreeMap},
	io,
	ops::ControlFlow,
	pin::{Pin, pin},
};

use futures_util::{Sink, SinkExt, Stream, StreamExt, TryFutureExt};
use tokio::{
	sync::mpsc,
	time::{Instant, sleep},
};
use tokio_util::sync::CancellationToken;
use tokio_websockets::proto::{CloseCode, Message as RawMessage};

use self::message::{EncodeMessageError, Message};
use super::{ServerMessage, ServerMonitorConfig};
use crate::{
	checksum::Checksum,
	database::{Database, DatabaseConnection, DatabaseError, DatabaseResult},
	event_queue::{self, Event},
	game::Game,
	maps::{self, Map},
	mode::Mode,
	players::{self, PlayerId, PlayerIp, PlayerName, PlayerPreferences},
	records::{self, CreatedRecord},
	servers::{self, ServerId, ServerSessionId},
	styles::Style,
};

#[derive(Debug)]
struct ConnectionState
{
	/// The server's ID
	server_id: ServerId,

	/// ID of this connection session
	session_id: ServerSessionId,

	#[debug(skip)]
	database: Database,

	/// The game the server is running
	game: Game,

	/// "map" from checksums to modes
	mode_checksums: Box<[(Checksum, Mode)]>,

	/// "map" from checksums to styles
	style_checksums: Box<[(Checksum, Style)]>,

	/// The map the server is currently hosting
	map: CurrentMap,

	/// Players currently playing on the server
	players: BTreeMap<PlayerId, ConnectedPlayer>,
}

impl ConnectionState
{
	fn connection_info(&self) -> servers::ConnectionInfo
	{
		servers::ConnectionInfo {
			current_map: match self.map {
				CurrentMap::Global(ref map) => map.name.as_str().into(),
				CurrentMap::NonGlobal { ref name } => name.clone(),
			},
			connected_players: self
				.players
				.values()
				.map(|player| servers::ConnectedPlayerInfo {
					id: player.id,
					name: player.name.clone(),
				})
				.collect(),
		}
	}
}

#[derive(Debug)]
enum CurrentMap
{
	Global(Map),
	NonGlobal
	{
		name: Box<str>,
	},
}

impl CurrentMap
{
	fn as_global(&self) -> Option<&Map>
	{
		match *self {
			CurrentMap::Global(ref map) => Some(map),
			CurrentMap::NonGlobal { .. } => None,
		}
	}

	fn from_maybe_global(map: Option<Map>, name: Box<str>) -> Self
	{
		map.map_or_else(|| Self::NonGlobal { name }, Self::Global)
	}
}

#[derive(Debug)]
struct ConnectedPlayer
{
	id: PlayerId,
	name: PlayerName,
	ip_address: PlayerIp,
	is_banned: bool,
	preferences: PlayerPreferences,
}

#[derive(Debug, Display, Error, From)]
pub(super) enum ServerTaskError
{
	#[from]
	Io(io::Error),

	#[from]
	EncodeMessage(EncodeMessageError),

	#[from(DatabaseError, sqlx::Error)]
	Database(DatabaseError),
}

#[tracing::instrument(skip(socket, database, cancellation_token, command_rx), err)]
pub(super) async fn main_loop<S>(
	mut socket: Pin<&mut S>,
	server_id: ServerId,
	database: Database,
	cancellation_token: CancellationToken,
	config: ServerMonitorConfig,
	mut command_rx: mpsc::Receiver<ServerMessage>,
) -> Result<(), ServerTaskError>
where
	S: Stream<Item = io::Result<RawMessage>>,
	S: Sink<RawMessage, Error = io::Error>,
{
	let ControlFlow::Continue(mut state) = handshake::perform(
		socket.as_mut(),
		server_id,
		database,
		&cancellation_token,
		config.handshake_timeout,
	)
	.await?
	else {
		return Ok(());
	};

	event_queue::dispatch(Event::ServerConnected(state.connection_info()));

	let mut heartbeat_timeout = pin!(sleep(config.heartbeat_interval));

	loop {
		select! {
			() = cancellation_token.cancelled() => {
				tracing::debug!("closing connection due to server shutdown");
				socket.as_mut().send(shutdown_message()).await?;
				break Ok(());
			},

			() = &mut heartbeat_timeout => {
				tracing::debug!("closing connection due to timeout");
				socket.as_mut().send(timeout_message()).await?;
				break Ok(());
			},

			Some(recv_result) = socket.next() => {
				heartbeat_timeout
					.as_mut()
					.reset(Instant::now() + config.heartbeat_interval);

				match recv_result {
					Ok(message) => {
						if let Some(message) = decode_message(socket.as_mut(), message).await? {
							handle_message(socket.as_mut(), &mut state, message).await?;
						}
					},
					Err(err) => return Err(err.into()),
				}
			},

			Some(command) = command_rx.recv() => {
				if handle_command(socket.as_mut(), &state, command).await?.is_break() {
					break Ok(());
				}
			},
		};

		// Make sure all `socket.feed(…)`'ed messages are actually sent
		socket.as_mut().flush().await?;
	}
}

/// Decodes an incoming message.
///
/// Messages that could not be decoded into something useful are handled
/// appropriately by this function.
#[tracing::instrument(level = "debug", skip(socket), ret(level = "debug"), err)]
async fn decode_message<S>(
	mut socket: Pin<&mut S>,
	message: RawMessage,
) -> Result<Option<Message<message::Incoming>>, ServerTaskError>
where
	S: Sink<RawMessage, Error = io::Error>,
{
	if message.is_ping() {
		tracing::debug!(payload.size = message.as_payload().len(), "ping");
		Ok(None)
	} else if message.is_pong() {
		tracing::debug!(payload.size = message.as_payload().len(), "pong?");
		Ok(None)
	} else if let Some((code, reason)) = message.as_close() {
		tracing::debug!(code = u16::from(code), reason, "client closed the connection");
		Ok(None)
	} else {
		tracing::debug!(
			payload.size = message.as_payload().len(),
			text = message.is_text(),
			"decoding message",
		);

		match Message::<message::Incoming>::decode(&message) {
			Ok(message) => Ok(Some(message)),
			Err(err) => {
				socket
					.send(Message::error(0, &err).encode()?)
					.map_ok(|()| None)
					.map_err(ServerTaskError::from)
					.await
			},
		}
	}
}

/// Handles an incoming message.
#[tracing::instrument(level = "debug", skip(socket), err)]
async fn handle_message<S>(
	mut socket: Pin<&mut S>,
	state: &mut ConnectionState,
	message: Message<message::Incoming>,
) -> Result<(), ServerTaskError>
where
	S: Sink<RawMessage, Error = io::Error>,
{
	let (message_id, payload) = message.into_parts();

	match payload {
		message::Incoming::MapChange { name } => {
			tracing::debug!(map = &*name, "server changed map");

			let map = {
				let mut conn = state.database.acquire_connection().await?;
				maps::get_by_name(&*name).exec(&mut conn).await?
			};

			let message =
				Message::new(message_id, message::Outgoing::MapChangeAck { map: map.as_ref() });

			socket.feed(message.encode()?).await?;
			state.map = CurrentMap::from_maybe_global(map, name);
		},

		message::Incoming::PlayerJoin { id, name, ip_address } => {
			tracing::debug!(%id, %name, "player joined");

			let player = state
				.database
				.in_transaction(async |conn| {
					on_player_join(id)
						.name(name)
						.ip_address(ip_address)
						.exec(&*state, conn)
						.await
				})
				.await?;

			match state.players.entry(id) {
				btree_map::Entry::Vacant(entry) => {
					let player = &*entry.insert(player);
					let reply = Message::new(message_id, message::Outgoing::PlayerJoinAck {
						player_id: player.id,
						is_banned: player.is_banned,
						preferences: &player.preferences,
					});

					socket.as_mut().feed(reply.encode()?).await?;
				},
				btree_map::Entry::Occupied(entry) => {
					tracing::warn!(
						old_player = ?entry.get(),
						"received player-join event while player was still in cache",
					);
				},
			}
		},

		message::Incoming::PlayerLeave { id, name, preferences } => {
			tracing::debug!(%id, %name, "player left");

			if let Some(player) = state.players.remove(&id) {
				state
					.database
					.in_transaction(async |conn| {
						on_player_leave(id)
							.name(name)
							.ip_address(player.ip_address)
							.preferences(preferences)
							.exec(&*state, conn)
							.await
					})
					.await?;
			} else {
				tracing::warn!(
					%id,
					%name,
					"received player-leave event while player was not in cache",
				);
			}
		},

		message::Incoming::NewRecord {
			player_id,
			course_id,
			mode_checksum,
			style_checksums,
			time,
			teleports,
		} => {
			let Some(filter_id) = state.map.as_global().and_then(|map| {
				let filters = map.courses.get(&course_id).map(|course| &course.filters)?;
				let filter = state
					.mode_checksums
					.iter()
					.find(|&&(checksum, _)| checksum == mode_checksum)
					.and_then(|&(_, mode)| match mode {
						Mode::Vanilla => filters.cs2.as_ref().map(|filters| &filters.vnl),
						Mode::Classic => filters.cs2.as_ref().map(|filters| &filters.ckz),
						Mode::KZTimer => filters.csgo.as_ref().map(|filters| &filters.kzt),
						Mode::SimpleKZ => filters.csgo.as_ref().map(|filters| &filters.skz),
						Mode::VanillaCSGO => filters.csgo.as_ref().map(|filters| &filters.vnl),
					})?;

				Some(filter.id)
			}) else {
				#[derive(Debug, Display, Error)]
				#[display("cannot submit record on invalid filter")]
				struct InvalidFilter;

				let message = Message::error(message_id, &InvalidFilter);
				socket.feed(message.encode()?).await?;
				return Ok(());
			};

			let styles = style_checksums.iter().filter_map(|&style_checksum| {
				state
					.style_checksums
					.iter()
					.find(|&&(checksum, _)| checksum == style_checksum)
					.map(|&(_, style)| style)
			});

			match state
				.database
				.in_transaction(async |conn| {
					records::create(filter_id, player_id)
						.session_id(state.session_id)
						.time(time)
						.teleports(teleports)
						.styles(styles.collect())
						.exec(conn)
						.await
				})
				.await
			{
				Ok(CreatedRecord { id, ranked_data }) => {
					event_queue::dispatch(Event::RecordSubmitted { record_id: id });

					let message = Message::new(message_id, message::Outgoing::NewRecordAck {
						record_id: id,
						ranked_data: ranked_data.as_ref(),
					});

					socket.feed(message.encode()?).await?;
				},
				Err(err) => {
					let message = Message::error(message_id, &err);

					socket.feed(message.encode()?).await?;
				},
			}
		},
	}

	Ok(())
}

/// Handles a command from the [`ServerMonitor`].
#[tracing::instrument(level = "debug", skip(socket), err)]
async fn handle_command<S>(
	mut socket: Pin<&mut S>,
	state: &ConnectionState,
	command: ServerMessage,
) -> Result<ControlFlow<()>, ServerTaskError>
where
	S: Sink<RawMessage, Error = io::Error>,
{
	match command {
		ServerMessage::Disconnect => {
			tracing::warn!("disconnecting");

			return socket
				.send(disconnect_message())
				.map_ok(ControlFlow::Break)
				.map_err(ServerTaskError::from)
				.await;
		},
		ServerMessage::WantConnectionInfo { response_tx } => {
			tracing::trace!("transmitting connection info");

			let _ = response_tx.send(Some(state.connection_info()));
		},

		ServerMessage::BroadcastChatMessage { message, response_tx } => {
			tracing::trace!("telling server to broadcast message");

			if let Err(err) = try {
				let message =
					Message::new(0, message::Outgoing::BroadcastChatMessage { message: &message });

				socket.as_mut().feed(message.encode()?).await?
			} {
				let _ = response_tx.send(false);
				return Err(err);
			}

			let _ = response_tx.send(true);
		},
	}

	Ok(ControlFlow::Continue(()))
}

/// Handles the [`PlayerJoin`] message.
///
/// [`PlayerJoin`]: message::Incoming::PlayerJoin
#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn on_player_join(
	#[builder(start_fn)] id: PlayerId,
	#[builder(finish_fn)] state: &ConnectionState,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: PlayerName,
	ip_address: PlayerIp,
) -> DatabaseResult<ConnectedPlayer>
{
	sqlx::query!(
		"INSERT INTO Players (id, name, ip_address)
		 VALUES (?, ?, ?)
		 ON DUPLICATE KEY
		 UPDATE name = VALUES(name),
		        ip_address = VALUES(ip_address)",
		id,
		name,
		ip_address,
	)
	.execute(conn.as_raw())
	.await?;

	let is_banned = players::is_banned(id).exec(&mut *conn).await?;
	let preferences = players::get_preferences(id)
		.game(state.game)
		.exec(conn)
		.await?
		.unwrap_or_else(|| {
			tracing::warn!("`players::get_preferences()` returned `None` after creating player");
			PlayerPreferences::default()
		});

	event_queue::dispatch(Event::PlayerJoin {
		server_id: state.server_id,
		player: servers::ConnectedPlayerInfo { id, name: name.clone() },
	});

	Ok(ConnectedPlayer { id, name, ip_address, is_banned, preferences })
}

/// Handles the [`PlayerLeave`] message.
///
/// [`PlayerLeave`]: message::Incoming::PlayerLeave
#[tracing::instrument(level = "debug", skip(conn), ret(level = "debug"), err)]
#[builder(finish_fn = exec)]
async fn on_player_leave(
	#[builder(start_fn)] id: PlayerId,
	#[builder(finish_fn)] state: &ConnectionState,
	#[builder(finish_fn)] conn: &mut DatabaseConnection<'_, '_>,
	name: PlayerName,
	ip_address: PlayerIp,
	preferences: PlayerPreferences,
) -> DatabaseResult<()>
{
	let (conn, query) = conn.as_parts();

	query.reset();
	query.push("UPDATE Players SET name = ");
	query.push_bind(name);
	query.push(", ip_address = ");
	query.push_bind(ip_address);
	query.push(", ");
	query.push(match state.game {
		Game::CS2 => "cs2_preferences",
		Game::CSGO => "csgo_preferences",
	});
	query.push(" = ");
	query.push_bind(preferences);
	query.push(" WHERE id = ");
	query.push(id);

	query.build().execute(conn).await?;

	event_queue::dispatch(Event::PlayerLeave { server_id: state.server_id, player_id: id });

	Ok(())
}

pub(super) fn internal_server_error() -> RawMessage
{
	RawMessage::close(Some(CloseCode::INTERNAL_SERVER_ERROR), "server encountered an error")
}

fn shutdown_message() -> RawMessage
{
	RawMessage::close(Some(CloseCode::GOING_AWAY), "server shutting down")
}

fn timeout_message() -> RawMessage
{
	RawMessage::close(Some(CloseCode::POLICY_VIOLATION), "exceeded heartbeat timeout")
}

fn disconnect_message() -> RawMessage
{
	RawMessage::close(Some(CloseCode::NORMAL_CLOSURE), "")
}
