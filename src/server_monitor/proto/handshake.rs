use {
	super::{ConnectionState, CurrentMap, on_player_join},
	crate::{
		checksum::Checksum,
		database::{self, DatabaseError},
		maps::{self, Map},
		mode::Mode,
		players::{PlayerId, PlayerIp, PlayerName},
		plugin,
		server_monitor::Config,
		servers::{self, ServerId},
		styles::Style,
		time::Seconds,
	},
	axum_tws::WebSocketError,
	futures_util::{Sink, SinkExt, Stream, StreamExt, TryStreamExt},
	serde::{Deserialize, Serialize},
	std::{
		collections::{BTreeMap, HashMap},
		pin::{Pin, pin},
		time::Duration,
	},
	tokio::time::sleep,
	tokio_websockets::Message as RawMessage,
};

#[derive(Debug, Display, Error, From)]
#[display("handshake error: {_variant}")]
pub(super) enum HandshakeError
{
	#[from]
	Io(WebSocketError),

	#[from]
	DatabaseError(DatabaseError),

	#[display("client closed the connection")]
	ClientClosedConnection,

	#[display("exceeded {timeout:?} timeout")]
	#[from(ignore)]
	TimeoutExceeded
	{
		#[error(ignore)]
		timeout: Duration,
	},

	#[display("failed to decode 'Hello' payload: {_0}")]
	DecodeHello(serde_json::Error),

	#[display("failed to encode 'HelloAck' payload: {_0}")]
	EncodeHelloAck(serde_json::Error),

	#[display("invalid plugin version")]
	InvalidPluginVersion,
}

/// Payload sent by the client during the handshake
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
struct Hello
{
	/// Checksum of the CS2KZ plugin binary the server is currently running
	plugin_checksum: Checksum,

	/// Name of the map the server is currently hosting
	current_map: Box<str>,

	/// Players currently connected to the server
	connected_players: BTreeMap<PlayerId, ConnectedPlayer>,
}

#[derive(Debug, Deserialize)]
struct ConnectedPlayer
{
	name: PlayerName,
	ip_address: PlayerIp,
}

/// Payload sent by the server during the handshake
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
struct HelloAck<'a>
{
	/// Interval at which the client should send heartbeat pings
	heartbeat_interval: Seconds,

	/// Checksums for all modes the server is allowed to submit records for
	mode_checksums: &'a HashMap<Mode, Checksum>,

	/// Checksums for all styles the server is allowed to submit records for
	style_checksums: &'a HashMap<Style, Checksum>,

	/// Detailed information about the map the server is currently hosting
	map_info: Option<&'a Map>,

	/// Detailed information about the [`connected_players`] the server told us
	/// about
	///
	/// [`connected_players`]: Hello::connected_players
	#[serde(serialize_with = "HelloAck::serialize_player_details")]
	player_details: &'a BTreeMap<PlayerId, super::ConnectedPlayer>,
}

impl<'a> HelloAck<'a>
{
	fn serialize_player_details<S>(
		player_details: &'a BTreeMap<PlayerId, super::ConnectedPlayer>,
		serializer: S,
	) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeMap;

		struct SerializePlayerId<'a>(&'a PlayerId);

		impl Serialize for SerializePlayerId<'_>
		{
			fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
			where
				S: serde::Serializer,
			{
				self.0.as_ref().serialize_u64_stringified(serializer)
			}
		}

		let mut serializer = serializer.serialize_map(Some(player_details.len()))?;

		for (player_id, player) in player_details {
			serializer.serialize_entry(&SerializePlayerId(player_id), player)?;
		}

		serializer.end()
	}
}

#[instrument(skip(socket, config, database), ret(level = "debug"), err(level = "debug"))]
pub(super) async fn perform<S>(
	mut socket: Pin<&mut S>,
	config: &Config,
	database: &database::ConnectionPool,
	server_id: ServerId,
) -> Result<ConnectionState, HandshakeError>
where
	S: Stream<Item = Result<RawMessage, WebSocketError>> + Sink<RawMessage, Error = WebSocketError>,
{
	let mut timeout = pin!(sleep(config.handshake_timeout));

	let Hello { plugin_checksum, current_map, connected_players } = loop {
		select! {
			() = &mut timeout => {
				debug!("exceeded handshake timeout");
				return Err(HandshakeError::TimeoutExceeded { timeout: config.handshake_timeout });
			},

			Some(message) = socket.next() => {
				let message = message?;
				let payload = message.as_payload();

				if message.is_ping() {
					trace!(payload.size = payload.len(), "ping");
					continue;
				}

				if message.is_pong() {
					trace!(payload.size = payload.len(), "pong");
					continue;
				}

				if let Some((code, reason)) = message.as_close() {
					trace!(?code, reason, "client closed the connection");
					return Err(HandshakeError::ClientClosedConnection);
				}

				trace!(payload.size = payload.len(), "decoding message");

				break serde_json::from_slice::<Hello>(&payload[..])
					.map_err(HandshakeError::DecodeHello)?;
			},
		};
	};

	let mut db_conn = database.acquire().await?;

	let (version_id, game, os) = plugin::validate_checksum(&plugin_checksum)
		.exec(&mut db_conn)
		.await?
		.ok_or(HandshakeError::InvalidPluginVersion)?;

	let mode_checksums = plugin::get_mode_checksums(version_id)
		.exec(&mut db_conn)
		.map_ok(|(mode, checksums)| (mode, checksums[os]))
		.try_collect::<HashMap<_, _>>()
		.await?;

	let style_checksums = plugin::get_style_checksums(version_id)
		.exec(&mut db_conn)
		.map_ok(|(style, checksums)| (style, checksums[os]))
		.try_collect::<HashMap<_, _>>()
		.await?;

	let session_id = db_conn
		.in_transaction(async |conn| {
			servers::create_session(server_id)
				.plugin_version_id(version_id)
				.exec(conn)
				.await
		})
		.await?;

	let current_map = match maps::get_by_name(&current_map).exec(&mut db_conn).await? {
		Some(map) => CurrentMap::Known(map),
		None => CurrentMap::Unknown { name: current_map },
	};

	let players = {
		let mut players = BTreeMap::default();

		for (player_id, ConnectedPlayer { name, ip_address }) in connected_players {
			let player = on_player_join(player_id, game)
				.name(name)
				.ip_address(ip_address)
				.exec(&mut db_conn)
				.await?;

			players.insert(player_id, player);
		}

		players
	};

	let ack = HelloAck {
		heartbeat_interval: config.heartbeat_interval.into(),
		mode_checksums: &mode_checksums,
		style_checksums: &style_checksums,
		map_info: match current_map {
			CurrentMap::Known(ref map) => Some(map),
			CurrentMap::Unknown { .. } => None,
		},
		player_details: &players,
	};

	match serde_json::to_string(&ack).map(RawMessage::text) {
		Ok(message) => socket.send(message).await?,
		Err(err) => return Err(HandshakeError::EncodeHelloAck(err)),
	}

	let mode_checksums = mode_checksums
		.into_iter()
		.map(|(mode, checksum)| (checksum, mode))
		.collect::<HashMap<_, _>>();

	let style_checksums = style_checksums
		.into_iter()
		.map(|(style, checksum)| (checksum, style))
		.collect::<HashMap<_, _>>();

	Ok(ConnectionState {
		server_id,
		version_id,
		game,
		session_id,
		current_map,
		players,
		mode_checksums,
		style_checksums,
	})
}
