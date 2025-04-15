use {
	self::{
		handshake::HandshakeError,
		message::{Message, MessageId},
	},
	crate::{
		checksum::Checksum,
		database::{self, DatabaseResult},
		event_queue::{self, Event},
		game::Game,
		maps::{self, Filters, Map},
		mode::Mode,
		players::{self, PlayerId, PlayerIp, PlayerName, PlayerPreferences},
		plugin::PluginVersionId,
		points::PointsDaemonHandle,
		records,
		server_monitor::{Config, ServerMessage},
		servers::{self, ServerId, ServerSessionId},
		styles::{Style, Styles},
	},
	axum_tws::WebSocketError,
	color_eyre::eyre::{self, Context},
	futures_util::{Sink, SinkExt, Stream, StreamExt},
	serde::Serialize,
	std::{
		collections::{
			HashMap,
			btree_map::{self, BTreeMap},
		},
		error::Error,
		ops::ControlFlow,
		pin::pin,
	},
	tokio::{
		sync::mpsc,
		time::{Instant, sleep},
	},
	tokio_websockets::{CloseCode, Message as RawMessage},
};

mod handshake;
mod message;

type WebSocketResult<T> = Result<T, WebSocketError>;
type OnSocketMessageResult = ControlFlow<WebSocketResult<()>, Option<RawMessage>>;

#[derive(Debug)]
struct ConnectionState
{
	/// ID of the connected server
	server_id: ServerId,

	/// ID of the plugin version the server is currently running
	#[expect(dead_code, reason = "captured by tracing")]
	version_id: PluginVersionId,

	/// The game the server is currently running
	game: Game,

	/// ID of this connection session
	session_id: ServerSessionId,

	/// The map the server is currently hosting
	current_map: CurrentMap,

	/// The players currently connected to the server
	players: BTreeMap<PlayerId, ConnectedPlayer>,

	/// Checksums of the modes relevant for this connection
	mode_checksums: HashMap<Checksum, Mode>,

	/// Checksums of the styles relevant for this connection
	style_checksums: HashMap<Checksum, Style>,
}

impl ConnectionState
{
	fn connection_info(&self) -> servers::ConnectionInfo
	{
		servers::ConnectionInfo {
			current_map: match self.current_map {
				CurrentMap::Known(ref map) => map.name.as_str().into(),
				CurrentMap::Unknown { ref name } => name.clone(),
			},
			connected_players: self
				.players
				.iter()
				.map(|(&id, player)| servers::ConnectedPlayerInfo { id, name: player.name.clone() })
				.collect(),
		}
	}
}

#[derive(Debug)]
enum CurrentMap
{
	/// The API knows about the map
	Known(Map),

	/// The API does not know about the map
	Unknown
	{
		/// The map's name according to the server
		name: Box<str>,
	},
}

#[derive(Debug, Serialize)]
struct ConnectedPlayer
{
	/// The player's name when they joined the server
	#[serde(skip_serializing)]
	name: PlayerName,

	/// The player's IP address
	#[serde(skip_serializing)]
	#[expect(dead_code, reason = "captured by tracing")]
	ip_address: PlayerIp,

	/// The player's in-game preferences
	preferences: PlayerPreferences,

	/// Whether the player is currently banned
	is_banned: bool,
}

#[instrument(skip(socket, rx), err)]
pub(in crate::server_monitor) async fn serve_connection<S>(
	socket: S,
	mut rx: mpsc::Receiver<ServerMessage>,
	config: Config,
	database: database::ConnectionPool,
	points_daemon: PointsDaemonHandle,
	server_id: ServerId,
) -> WebSocketResult<()>
where
	S: Stream<Item = WebSocketResult<RawMessage>> + Sink<RawMessage, Error = WebSocketError>,
{
	let mut socket = pin!(socket);
	let mut heartbeat_timeout = pin!(sleep(config.heartbeat_interval));
	let mut state = match handshake::perform(socket.as_mut(), &config, &database, server_id).await {
		Ok(state) => state,
		Err(HandshakeError::Io(err)) => return Err(err),
		Err(HandshakeError::DatabaseError(err)) => {
			error!(error = &err as &dyn Error, "encountered database error during handshake");
			return socket
				.send(RawMessage::close(
					Some(CloseCode::INTERNAL_SERVER_ERROR),
					"something went wrong",
				))
				.await;
		},
		Err(HandshakeError::ClientClosedConnection) => return Ok(()),
		Err(
			err @ (HandshakeError::DecodeHello(_)
			| HandshakeError::EncodeHelloAck(_)
			| HandshakeError::TimeoutExceeded { .. }
			| HandshakeError::InvalidPluginVersion),
		) => {
			return socket
				.send(RawMessage::close(Some(CloseCode::POLICY_VIOLATION), &err.to_string()))
				.await;
		},
	};

	heartbeat_timeout
		.as_mut()
		.reset(Instant::now() + config.heartbeat_interval);

	event_queue::dispatch(Event::ServerConnected {
		id: state.server_id,
		connection_info: state.connection_info(),
	});

	loop {
		select! {
			() = &mut heartbeat_timeout => {
				debug!("exceeded heartbeat timeout");
				return socket
					.send(RawMessage::close(Some(CloseCode::POLICY_VIOLATION), "exceeded heartbeat timeout"))
					.await;
			},

			Some(message) = rx.recv() => match message {
				ServerMessage::Disconnect { response_tx } => {
					let _guard = crate::util::drop_guard(move || {
						let _ = response_tx.send(true);
					});

					socket
						.send(RawMessage::close(Some(CloseCode::NORMAL_CLOSURE), ""))
						.await?;

					break Ok(());
				},

				ServerMessage::WantConnectionInfo { response_tx } => {
					let _ = response_tx.send(Some(state.connection_info()));
				},

				ServerMessage::BroadcastMessage { message, response_tx } => 'scope: {
					let message = Message::new(message::Outgoing::BroadcastMessage {
						message: &message,
					});

					let Some(message) = message.encode_lossy() else {
						let _ = response_tx.send(false);
						break 'scope;
					};

					match socket.feed(message).await {
						Ok(()) => {
							let _ = response_tx.send(true);
						},
						Err(err) => {
							let _ = response_tx.send(false);
							return Err(err);
						},
					}
				},
			},

			Some(message) = socket.next() => {
				heartbeat_timeout.as_mut().reset(Instant::now() + config.heartbeat_interval);

				match on_socket_message(&database, &points_daemon, &mut state, message?).await {
					OnSocketMessageResult::Break(res) => break res,
					OnSocketMessageResult::Continue(maybe_reply) => {
						if let Some(reply) = maybe_reply {
							socket.feed(reply).await?;
						}
					},
				}
			},
		};

		socket.flush().await?;
	}
}

/// Callback for receiving a WebSocket message
#[instrument(skip(database), ret(level = "debug"))]
async fn on_socket_message(
	database: &database::ConnectionPool,
	points_daemon: &PointsDaemonHandle,
	state: &mut ConnectionState,
	message: RawMessage,
) -> OnSocketMessageResult
{
	let payload = message.as_payload();

	if message.is_ping() {
		trace!(payload.size = payload.len(), "ping");
		return OnSocketMessageResult::Continue(None);
	}

	if message.is_pong() {
		trace!(payload.size = payload.len(), "pong");
		return OnSocketMessageResult::Continue(None);
	}

	if let Some((code, reason)) = message.as_close() {
		trace!(?code, reason, "client closed the connection");
		return OnSocketMessageResult::Break(Ok(()));
	}

	trace!(payload.size = payload.len(), "decoding message");

	OnSocketMessageResult::Continue(match Message::<message::Incoming>::decode(&payload[..]) {
		Ok(message) => {
			let (message_id, payload) = message.into_parts();
			debug!(id = message_id.as_u64(), ?payload, "decoded message");

			match handle_incoming_payload(database, points_daemon, state, message_id, payload).await
			{
				Ok(maybe_reply) => maybe_reply,
				Err(err) => {
					tracing::error!(err = format_args!("{err:#}"));
					let reply = Message::reply(message_id, message::Outgoing::Error {
						error: err.as_ref(),
					});

					reply.encode_lossy()
				},
			}
		},
		Err(err) => Message::<message::Outgoing<'_>>::from(&err).encode_lossy(),
	})
}

#[instrument(skip(database), ret(level = "debug"))]
async fn handle_incoming_payload(
	database: &database::ConnectionPool,
	points_daemon: &PointsDaemonHandle,
	state: &mut ConnectionState,
	message_id: MessageId,
	payload: message::Incoming,
) -> eyre::Result<Option<RawMessage>>
{
	match payload {
		message::Incoming::MapChanged { name } => {
			debug!(name, "changed map");

			state.current_map = {
				let mut db_conn = database.acquire().await?;

				match maps::get_by_name(&name).exec(&mut db_conn).await? {
					Some(map) => CurrentMap::Known(map),
					None => CurrentMap::Unknown { name },
				}
			};

			let reply = Message::reply(message_id, message::Outgoing::MapChangedAck {
				map_info: match state.current_map {
					CurrentMap::Known(ref map) => Some(map),
					CurrentMap::Unknown { .. } => None,
				},
			});

			Ok(reply.encode_lossy())
		},

		message::Incoming::PlayerJoin { id, name, ip_address } => {
			let player = database
				.in_transaction(async |conn| {
					on_player_join(id, state.game)
						.name(name)
						.ip_address(ip_address)
						.exec(conn)
						.await
				})
				.await
				.wrap_err("`on_player_join` failed")?;

			event_queue::dispatch(Event::PlayerJoin {
				server_id: state.server_id,
				player: servers::ConnectedPlayerInfo { id, name: player.name.clone() },
			});

			match state.players.entry(id) {
				btree_map::Entry::Vacant(entry) => {
					let player = entry.insert(player);
					debug!(%id, name = %player.name, "player joined the server");

					let reply = Message::reply(message_id, message::Outgoing::PlayerJoinAck {
						preferences: &player.preferences,
						is_banned: player.is_banned,
					});

					Ok(reply.encode_lossy())
				},
				btree_map::Entry::Occupied(mut entry) => {
					let old = entry.insert(player);
					let new = entry.get();
					warn!(
						%id,
						old.name = %old.name,
						new.name = %new.name,
						"player joined the server but is still in state map",
					);

					let reply = Message::reply(message_id, message::Outgoing::PlayerJoinAck {
						preferences: &new.preferences,
						is_banned: new.is_banned,
					});

					Ok(reply.encode_lossy())
				},
			}
		},

		message::Incoming::PlayerLeave { id, name, preferences } => {
			database
				.in_transaction(async |conn| {
					on_player_leave(id, state.game)
						.name(&name)
						.preferences(&preferences)
						.exec(conn)
						.await
				})
				.await?;

			event_queue::dispatch(Event::PlayerLeave { server_id: state.server_id, player_id: id });

			if let Some(old) = state.players.remove(&id) {
				debug!(%id, old_name = %old.name, new_name = %name, "player left the server");
			} else {
				warn!(%id, %name, "player left the server but not in state map");
			}

			Ok(None)
		},

		message::Incoming::SubmitRecord {
			course_local_id,
			mode_checksum,
			player_id,
			time,
			teleports,
			style_checksums,
		} => {
			let CurrentMap::Known(ref current_map) = state.current_map else {
				let reply = Message::reply(message_id, message::Outgoing::Error {
					error: &{
						#[derive(Debug, Display, Error)]
						#[display("cannot submit record on non-global map")]
						struct RecordSubmittedOnNonGlobalMap;
						RecordSubmittedOnNonGlobalMap
					},
				});

				return Ok(reply.encode_lossy());
			};

			let Some(course) = current_map
				.courses
				.values()
				.find(|course| course.local_id == course_local_id)
			else {
				let reply = Message::reply(message_id, message::Outgoing::Error {
					error: &{
						#[derive(Debug, Display, Error)]
						#[display("invalid course local ID")]
						struct InvalidCourseError;
						InvalidCourseError
					},
				});

				return Ok(reply.encode_lossy());
			};

			let Some(&mode) = state.mode_checksums.get(&mode_checksum) else {
				let reply = Message::reply(message_id, message::Outgoing::Error {
					error: &{
						#[derive(Debug, Display, Error)]
						#[display("invalid mode")]
						struct InvalidModeError;
						InvalidModeError
					},
				});

				return Ok(reply.encode_lossy());
			};

			debug_assert_eq!(state.game, mode.game());

			let filter = match (&course.filters, mode) {
				(Filters::CS2 { vnl, .. }, Mode::VanillaCS2)
				| (Filters::CSGO { vnl, .. }, Mode::VanillaCSGO) => vnl,
				(Filters::CS2 { ckz, .. }, Mode::Classic) => ckz,
				(Filters::CSGO { kzt, .. }, Mode::KZTimer) => kzt,
				(Filters::CSGO { skz, .. }, Mode::SimpleKZ) => skz,
				(filters, _) => unreachable!("{filters:#?}"),
			};

			let styles = style_checksums
				.iter()
				.filter_map(|checksum| state.style_checksums.get(checksum))
				.collect::<Styles>();

			let created_record = database
				.in_transaction(async |conn| {
					records::create(filter.id, player_id)
						.session_id(state.session_id)
						.time(time)
						.teleports(teleports)
						.styles(styles)
						.exec(conn)
						.await
				})
				.await
				.wrap_err("failed to create record")?;

			points_daemon.notify_record_submitted();
			event_queue::dispatch(Event::RecordSubmitted { record_id: created_record.id });

			let reply = Message::reply(message_id, message::Outgoing::SubmitRecordAck {
				record_id: created_record.id,
				ranked_data: created_record.ranked_data.as_ref(),
			});

			Ok(reply.encode_lossy())
		},
	}
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
async fn on_player_join(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: PlayerName,
	ip_address: PlayerIp,
) -> DatabaseResult<ConnectedPlayer>
{
	players::create(player_id)
		.name(&name)
		.ip_address(ip_address)
		.exec(db_conn)
		.await?;

	let preferences = players::get_preferences(player_id)
		.game(game)
		.exec(db_conn)
		.await?
		.unwrap_or_else(|| panic!("did not find player preferences after inserting player?"));

	let is_banned = players::is_banned(player_id).exec(db_conn).await?;

	Ok(ConnectedPlayer { name, ip_address, preferences, is_banned })
}

#[instrument(skip(db_conn))]
#[builder(finish_fn = exec)]
async fn on_player_leave(
	#[builder(start_fn)] player_id: PlayerId,
	#[builder(start_fn)] game: Game,
	#[builder(finish_fn)] db_conn: &mut database::Connection<'_, '_>,
	name: &PlayerName,
	preferences: &PlayerPreferences,
) -> DatabaseResult<()>
{
	let updated = players::update(player_id)
		.name(name)
		.preferences((preferences, game))
		.exec(db_conn)
		.await?;

	if !updated {
		warn!(id = %player_id, %name, "updated unknown player?");
	}

	Ok(())
}
