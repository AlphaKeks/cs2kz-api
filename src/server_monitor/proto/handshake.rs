use std::{
	collections::BTreeMap,
	io,
	ops::ControlFlow,
	pin::{Pin, pin},
	time::Duration,
};

use futures_util::{Sink, SinkExt, Stream, StreamExt, TryFutureExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tokio_websockets::proto::{CloseCode, Message as RawMessage};

use super::{
	ConnectionState,
	CurrentMap,
	ServerTaskError,
	message::{EncodeMessageError, Message},
	on_player_join,
	shutdown_message,
};
use crate::{
	checksum::Checksum,
	database::Database,
	game::Game,
	maps::{self, Map},
	players::{PlayerId, PlayerIp, PlayerName, PlayerPreferences},
	plugin::{self, PluginVersionId},
	servers::{self, ServerId},
};

/// First message sent by the client
#[derive(Debug, Deserialize)]
pub(super) struct Hello
{
	plugin_version_checksum: Checksum,
	current_map: Box<str>,
	current_players: Box<[AlreadyConnectedPlayer]>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AlreadyConnectedPlayer
{
	id: PlayerId,
	name: PlayerName,
	ip_address: PlayerIp,
}

/// Our response to [`Hello`]
#[derive(Debug, Serialize)]
pub(super) struct HelloAck<'a>
{
	map_info: Option<&'a Map>,
	players: BTreeMap<PlayerId, PlayerInfo<'a>>,
}

#[derive(Debug, Serialize)]
pub(super) struct PlayerInfo<'a>
{
	is_banned: bool,
	preferences: &'a PlayerPreferences,
}

/// Performs the handshake.
///
/// If `Ok(ControlFlow::Break(()))` is returned (e.g. because the given
/// `timeout_after` was exceeded), the caller should consider the connection
/// terminated.
///
/// If everything goes well, `Ok(ControlFlow::Continue(state))` is returned and
/// the connection can be continued with that state.
#[tracing::instrument(
	level = "debug",
	skip(socket, database, cancellation_token),
	ret(level = "debug"),
	err
)]
pub(super) async fn perform<S>(
	mut socket: Pin<&mut S>,
	server_id: ServerId,
	database: Database,
	cancellation_token: &CancellationToken,
	timeout_after: Duration,
) -> Result<ControlFlow<(), ConnectionState>, ServerTaskError>
where
	S: Stream<Item = io::Result<RawMessage>>,
	S: Sink<RawMessage, Error = io::Error>,
{
	let mut timeout = pin!(sleep(timeout_after));

	database
		.in_transaction(async |conn| {
			loop {
				let hello = select! {
					() = cancellation_token.cancelled() => {
						tracing::debug!("closing connection due to server shutdown");
						socket.as_mut().send(shutdown_message()).await?;
						return Ok(ControlFlow::Break(()));
					},

					() = &mut timeout => {
						tracing::debug!("closing connection due to timeout");
						socket.as_mut().send(timeout_message()).await?;
						return Ok(ControlFlow::Break(()));
					},

					Some(recv_result) = socket.next() => match recv_result {
						Ok(message) => match decode_hello(socket.as_mut(), message).await? {
							ControlFlow::Break(Some(hello)) => {
								tracing::trace!("received hello message");
								hello
							},
							ControlFlow::Break(None) => return Ok(ControlFlow::Break(())),
							ControlFlow::Continue(()) => continue,
						},
						Err(err) => return Err(err.into()),
					},
				};

				let Some((plugin_version_id, game, plugin_checksums)) = sqlx::query!(
					"SELECT
					   id AS `id: PluginVersionId`,
					   game AS `game: Game`,
					   linux_checksum AS `linux_checksum: Checksum`,
					   windows_checksum AS `windows_checksum: Checksum`
					 FROM PluginVersions
					 WHERE (linux_checksum = ? OR windows_checksum = ?)",
					hello.plugin_version_checksum,
					hello.plugin_version_checksum,
				)
				.fetch_optional(conn.as_raw())
				.map_ok(|maybe_row| {
					maybe_row.map(|row| {
						(row.id, row.game, plugin::Checksums {
							linux: row.linux_checksum,
							windows: row.windows_checksum,
						})
					})
				})
				.await?
				else {
					tracing::debug!("closing connection due to invalid plugin version");
					socket.as_mut().send(unauthorized_message()).await?;
					return Ok(ControlFlow::Break(()));
				};

				let session_id = servers::create_session(server_id)
					.plugin_version_id(plugin_version_id)
					.exec(&mut *conn)
					.await?;

				let mode_checksums = plugin::get_mode_checksums(plugin_version_id)
					.exec(&mut *conn)
					.map_ok(|(mode, checksums)| {
						let checksum = if hello.plugin_version_checksum == plugin_checksums.linux {
							checksums.linux
						} else {
							debug_assert_eq!(hello.plugin_version_checksum, plugin_checksums.windows);
							checksums.windows
						};

						(checksum, mode)
					})
					.try_collect::<Vec<_>>()
					.map_ok(Vec::into_boxed_slice)
					.await?;

				let style_checksums = plugin::get_style_checksums(plugin_version_id)
					.exec(&mut *conn)
					.map_ok(|(style, checksums)| {
						let checksum = if hello.plugin_version_checksum == plugin_checksums.linux {
							checksums.linux
						} else {
							debug_assert_eq!(hello.plugin_version_checksum, plugin_checksums.windows);
							checksums.windows
						};

						(checksum, style)
					})
					.try_collect::<Vec<_>>()
					.map_ok(Vec::into_boxed_slice)
					.await?;

				let map = {
					pin!(maps::get().name(&hello.current_map).limit(1).exec(&mut *conn))
						.try_next()
						.await?
				}
				.map_or(CurrentMap::NonGlobal { name: hello.current_map }, CurrentMap::Global);

				let mut state = ConnectionState {
					server_id,
					session_id,
					database: database.clone(),
					game,
					mode_checksums,
					style_checksums,
					map,
					players: BTreeMap::new(),
				};

				for player in hello.current_players {
					let player = on_player_join(player.id)
						.name(player.name)
						.ip_address(player.ip_address)
						.exec(&state, &mut *conn)
						.await?;

					state.players.insert(player.id, player);
				}

				let ack = serde_json::to_string(&HelloAck {
					map_info: match state.map {
						CurrentMap::Global(ref map) => Some(map),
						CurrentMap::NonGlobal { .. } => None,
					},
					players: state
						.players
						.iter()
						.map(|(&player_id, player)| {
							(player_id, PlayerInfo {
								is_banned: player.is_banned,
								preferences: &player.preferences,
							})
						})
						.collect(),
				})
				.map(RawMessage::text)
				.map_err(EncodeMessageError::from)?;

				socket.as_mut().send(ack).await?;
				tracing::trace!("sent hello ack");

				break Ok(ControlFlow::Continue(state));
			}
		})
		.await
}

/// Decodes an incoming message as [`Hello`].
///
/// A return value of [`ControlFlow::Break`] indicates that we have received
/// a message of the correct payload type, and whether a [`Hello`] was decoded
/// successfully. [`ControlFlow::Continue`] indicates we got some other message
/// (like ping) and should continue waiting.
#[tracing::instrument(level = "debug", skip(socket), ret(level = "debug"), err)]
async fn decode_hello<S>(
	mut socket: Pin<&mut S>,
	message: RawMessage,
) -> Result<ControlFlow<Option<Hello>>, ServerTaskError>
where
	S: Sink<RawMessage, Error = io::Error>,
{
	if message.is_ping() {
		tracing::debug!(payload.size = message.as_payload().len(), "ping");
		Ok(ControlFlow::Continue(()))
	} else if message.is_pong() {
		tracing::debug!(payload.size = message.as_payload().len(), "pong?");
		Ok(ControlFlow::Continue(()))
	} else if let Some((code, reason)) = message.as_close() {
		tracing::debug!(code = u16::from(code), reason, "client closed the connection");
		Ok(ControlFlow::Break(None))
	} else {
		tracing::debug!(
			payload.size = message.as_payload().len(),
			text = message.is_text(),
			"decoding message",
		);

		match serde_json::from_slice::<Hello>(&message.as_payload()[..]) {
			Ok(hello) => Ok(ControlFlow::Break(Some(hello))),
			Err(err) => {
				socket
					.send(Message::error(0, &err).encode()?)
					.map_ok(ControlFlow::Continue)
					.map_err(ServerTaskError::from)
					.await
			},
		}
	}
}

fn timeout_message() -> RawMessage
{
	RawMessage::close(Some(CloseCode::POLICY_VIOLATION), "exceeded handshake timeout")
}

fn unauthorized_message() -> RawMessage
{
	RawMessage::close(Some(CloseCode::POLICY_VIOLATION), "unauthorized")
}
