//! WebSocket connection abstractions.
//!
//! This module contains the [`Connection`] type, which acts as a wrapper around
//! a raw WebSocket stream. It handles timeouts, cancellation, and
//! encoding/decoding messages.

use std::collections::HashMap;
use std::fmt;
use std::ops::ControlFlow;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use axum::extract::ws;
use cs2kz::SteamID;
use futures::{Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use tap::Tap;
use tokio::select;
use tokio_util::sync::CancellationToken;

use super::message::{self, DecodeMessageError, Message};
use super::{CloseReason, PlayerInfo};
use crate::services::maps::{FetchMapRequest, MapService};
use crate::services::players::{FetchPlayerPreferencesRequest, PlayerService, UpdatePlayerRequest};
use crate::services::servers::ServerID;
use crate::time::DurationExt;
use crate::util::MapIdentifier;

mod errors;
pub use errors::{
	EstablishConnectionError,
	HandshakeError,
	ReceiveMessageError,
	SendMessageError,
	ServeConnectionError,
};

mod handshake;
use handshake::{Hello, HelloAck};

mod timeout;
use timeout::Timeout;

/// A live WebSocket connection.
pub struct Connection<S>
{
	/// The underlying I/O stream.
	stream: S,

	/// How long we should wait for the client to send a heartbeat before
	/// closing the connection due to idling.
	heartbeat_interval: Duration,

	/// A cancellation token so we can gracefully close connections when the
	/// server is shutting down.
	cancellation_token: CancellationToken,

	/// ID of the server currently connected to us.
	server_id: ServerID,

	/// Service for fetching map information.
	map_service: MapService,

	/// Service for updating player information.
	player_service: PlayerService,

	/// The players currently connected to the server.
	connected_players: HashMap<SteamID, PlayerInfo>,

	/// The total amount of online players (may include bots for example).
	total_players: u64,

	/// The amount of player slots on the server.
	max_players: u64,
}

impl<S, E> Connection<S>
where
	S: Stream<Item = Result<ws::Message, E>>,
	S: Sink<ws::Message>,
	S: Send + Unpin,
	E: std::error::Error + Send + Sync + 'static,
	<S as Sink<ws::Message>>::Error: std::error::Error + Send + Sync + 'static,
{
	/// Establishes a new connection.
	#[tracing::instrument(err(Debug, level = "debug"), skip(stream, cancellation_token))]
	pub async fn establish(
		stream: S,
		heartbeat_interval: Duration,
		cancellation_token: CancellationToken,
		server_id: ServerID,
		map_service: MapService,
		player_service: PlayerService,
	) -> Result<Self, EstablishConnectionError>
	{
		let mut conn = Self {
			stream,
			heartbeat_interval,
			cancellation_token,
			server_id,
			map_service,
			player_service,
			connected_players: Default::default(),
			max_players: 0,
			total_players: 0,
		};

		conn.perform_handshake().await?;

		Ok(conn)
	}

	/// Serves the connection.
	#[tracing::instrument(err(Debug, level = "debug"), skip(self))]
	pub async fn serve(&mut self) -> Result<(), ServeConnectionError>
	{
		let mut timeout = Timeout::new(self.heartbeat_interval);
		let cancellation_token = self.cancellation_token.clone();

		loop {
			select! {
				biased;

				() = cancellation_token.cancelled() => {
					self.close(CloseReason::ServerShutdown).await;
					break Ok(());
				},

				() = timeout.wait() => {
					self.close(CloseReason::ClientTimeout).await;
					break Ok(());
				},

				result = self.recv_message() => match result {
					Err(ReceiveMessageError::ConnectionClosed { close_frame }) => {
						tracing::debug!(?close_frame, "client closed connection");
						break Ok(());
					},
					Err(ReceiveMessageError::DecodeMessage(DecodeMessageError::NotJson)) => {
						tracing::trace!("received heartbeat ping");
						timeout.reset();
					}
					Err(ReceiveMessageError::DecodeMessage(error)) => {
						self.send_message(Message::error(&error)).await?;
					},
					Err(ReceiveMessageError::Io(error)) => {
						self.send_message(Message::error(&*error)).await?;
						continue;
					},
					Ok(message) => match self.on_message(message).await {
						Ok(ControlFlow::Continue(None)) => continue,
						Ok(ControlFlow::Continue(Some(reply))) => {
							self.send_message(reply).await?;
						},
						Ok(ControlFlow::Break(reason)) => {
							self.close(reason).await;
							break Ok(());
						},
						Err(error) => {
							self.send_message(Message::error(&error)).await?;
						},
					},
				},
			}
		}
	}

	/// Closes the connection.
	#[tracing::instrument(skip(self))]
	pub async fn close(&mut self, reason: CloseReason)
	{
		if let Err(error) = self
			.stream
			.send(ws::Message::Close(Some(reason.as_close_frame())))
			.await
		{
			tracing::error!(?error, "failed to send close frame");
		}
	}

	/// Performs the initial handshake with the client.
	#[tracing::instrument(err(Debug, level = "debug"), skip(self))]
	async fn perform_handshake(&mut self) -> Result<Hello, HandshakeError>
	{
		let timeout = tokio::time::sleep(Duration::MINUTE);
		tokio::pin!(timeout);

		loop {
			select! {
				() = &mut timeout => {
					break Err(HandshakeError::Timeout);
				},

				Some(result) = self.stream.next() => match result {
					Err(io_error) => {
						break Err(HandshakeError::Io(io_error.into()));
					},
					Ok(raw) => match Hello::decode(raw) {
						Err(DecodeMessageError::ParseJson(err)) => {
							let message = Message::error(&err);
							let encoded = message.encode()?;

							self.stream
								.send(encoded)
								.await
								.map_err(|err| HandshakeError::Io(err.into()))?;

							continue;
						},
						Err(DecodeMessageError::NotJson) => {
							// ignore and try again
							continue;
						},
						Err(DecodeMessageError::ConnectionClosed { close_frame }) => {
							break Err(HandshakeError::ConnectionClosed { close_frame });
						},
						Ok(hello) => {
							tracing::info!(?hello, "received hello message");

							let ack = HelloAck::new(self.heartbeat_interval);
							let encoded = ack.encode()?;

							self.stream
								.send(encoded)
								.await
								.map_err(|err| HandshakeError::Io(err.into()))?;

							tracing::info!(?ack, "sent hello ack message");

							break Ok(hello);
						},
					},
				},

				else => break Err(HandshakeError::ConnectionClosed { close_frame: None }),
			}
		}
	}

	/// Callback for received messages.
	#[tracing::instrument(ret(level = "debug"), err(Debug, level = "debug"))]
	async fn on_message(
		&mut self,
		message: Message<message::Incoming>,
	) -> Result<ControlFlow<CloseReason, Option<Message<message::Outgoing>>>, ServeConnectionError>
	{
		use message::Incoming as M;

		match message.payload {
			M::MapChange { map_name } => {
				match self
					.map_service
					.fetch_map(FetchMapRequest { ident: MapIdentifier::Name(map_name) })
					.await
				{
					Ok(maybe_map) => {
						return Ok(ControlFlow::Continue(Some(Message {
							id: message.id,
							payload: message::Outgoing::MapInfo(maybe_map),
						})));
					}
					Err(error) => {
						return Ok(ControlFlow::Continue(Some(Message::error(&error).tap_mut(
							|msg| {
								msg.id = message.id;
							},
						))));
					}
				}
			}
			M::PlayerCountChange { authenticated_players, total_players, max_players } => {
				tracing::info! {
					authenticated_players = authenticated_players.len(),
					total_players,
					max_players,
					"player count changed",
				};

				self.connected_players.clear();
				self.connected_players.extend(
					authenticated_players
						.into_iter()
						.map(|player| (player.steam_id, player)),
				);

				self.total_players = total_players;
				self.max_players = max_players;
			}
			M::PlayerUpdate { player, preferences, session } => {
				tracing::info!(steam_id = %player.steam_id, "updating {}", player.name);
				self.player_service
					.update_player(UpdatePlayerRequest {
						player_id: player.steam_id,
						server_id: self.server_id,
						name: player.name,
						ip_address: player.ip_addr,
						preferences,
						session,
					})
					.await?;
			}
			M::GetPreferences { player_id } => {
				match self
					.player_service
					.fetch_player_preferences(FetchPlayerPreferencesRequest {
						identifier: player_id.into(),
					})
					.await
				{
					Ok(maybe_preferences) => {
						return Ok(ControlFlow::Continue(Some(Message {
							id: message.id,
							payload: message::Outgoing::Preferences {
								preferences: maybe_preferences.map(|x| x.preferences),
							},
						})));
					}
					Err(error) => {
						return Ok(ControlFlow::Continue(Some(Message::error(&error).tap_mut(
							|msg| {
								msg.id = message.id;
							},
						))));
					}
				}
			}
			M::GetMap { map_identifier } => {
				match self
					.map_service
					.fetch_map(FetchMapRequest { ident: map_identifier })
					.await
				{
					Ok(maybe_map) => {
						return Ok(ControlFlow::Continue(Some(Message {
							id: message.id,
							payload: message::Outgoing::MapInfo(maybe_map),
						})));
					}
					Err(error) => {
						return Ok(ControlFlow::Continue(Some(Message::error(&error).tap_mut(
							|msg| {
								msg.id = message.id;
							},
						))));
					}
				}
			}
		}

		Ok(ControlFlow::Continue(None))
	}
}

impl<S, E> Connection<S>
where
	S: Stream<Item = Result<ws::Message, E>>,
	S: Send + Unpin,
	E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
	/// Receives and decodes a message from the underlying socket.
	#[tracing::instrument(ret(level = "debug"), err(Debug, level = "debug"), skip(self))]
	pub async fn recv_message(&mut self)
	-> Result<Message<message::Incoming>, ReceiveMessageError>
	{
		self.try_next().await.and_then(|maybe_message| {
			maybe_message.ok_or(ReceiveMessageError::ConnectionClosed { close_frame: None })
		})
	}
}

impl<S> Connection<S>
where
	S: Sink<ws::Message>,
	S: Send + Unpin,
	S::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
	/// Encodes and sends a message over the underlying socket.
	#[tracing::instrument(err(Debug, level = "debug"), skip(self))]
	pub async fn send_message(
		&mut self,
		message: Message<message::Outgoing>,
	) -> Result<(), SendMessageError>
	{
		self.send(message).await
	}
}

impl<S, E> Stream for Connection<S>
where
	S: Stream<Item = Result<ws::Message, E>>,
	S: Send + Unpin,
	E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
	type Item = Result<Message<message::Incoming>, ReceiveMessageError>;

	fn poll_next(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Option<Self::Item>>
	{
		match Pin::new(&mut self.stream)
			.poll_next(cx)
			.map_err(|err| ReceiveMessageError::Io(err.into()))?
		{
			Poll::Pending => Poll::Pending,
			Poll::Ready(None) => {
				Poll::Ready(Some(Err(ReceiveMessageError::ConnectionClosed { close_frame: None })))
			}
			Poll::Ready(Some(raw)) => Poll::Ready(Some(Message::decode(raw).map_err(Into::into))),
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		self.stream.size_hint()
	}
}

impl<S> Sink<Message<message::Outgoing>> for Connection<S>
where
	S: Sink<ws::Message>,
	S: Send + Unpin,
	S::Error: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
	type Error = SendMessageError;

	fn poll_ready(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		Pin::new(&mut self.stream)
			.poll_ready(cx)
			.map_err(|err| SendMessageError::Io(err.into()))
	}

	fn start_send(
		mut self: Pin<&mut Self>,
		message: Message<message::Outgoing>,
	) -> Result<(), Self::Error>
	{
		let message = message.encode()?;

		Pin::new(&mut self.stream)
			.start_send(message)
			.map_err(|err| SendMessageError::Io(err.into()))
	}

	fn poll_flush(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		Pin::new(&mut self.stream)
			.poll_flush(cx)
			.map_err(|err| SendMessageError::Io(err.into()))
	}

	fn poll_close(
		mut self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		Pin::new(&mut self.stream)
			.poll_close(cx)
			.map_err(|err| SendMessageError::Io(err.into()))
	}
}

impl<S> fmt::Debug for Connection<S>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Connection")
			.field("server_id", &self.server_id)
			.field("heartbeat_interval", &self.heartbeat_interval)
			.field("connected_players", &self.connected_players.len())
			.field("total_players", &self.total_players)
			.field("max_players", &self.max_players)
			.finish()
	}
}
