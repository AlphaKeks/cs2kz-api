//! This module contains the relevant types for the initial connection
//! handshake.

use axum::extract::ws;
use serde::{Deserialize, Serialize};

use super::super::message::{DecodeMessageError, EncodeMessageError};
use crate::services::players::PlayerInfo;
use crate::services::plugin::PluginVersion;
use crate::time::Seconds;

/// This message is sent by client right after the connection has been
/// opened.
///
/// The server will respond with a [`HelloAck`] message.
#[derive(Debug, Deserialize)]
#[expect(dead_code, reason = "will be used later")]
pub struct Hello
{
	/// The cs2kz-metamod version the server is currently running.
	pub plugin_version: PluginVersion,

	/// The name of the map the server is currently hosting.
	pub current_map: String,

	/// List of players currently on the server.
	pub players: Vec<PlayerInfo>,
}

impl Hello
{
	/// Decodes a raw message we received from a client.
	#[tracing::instrument(err(Debug, level = "debug"))]
	pub fn decode(raw: ws::Message) -> Result<Self, DecodeMessageError>
	{
		match raw {
			ws::Message::Text(text) => serde_json::from_str(&text).map_err(Into::into),
			ws::Message::Binary(bytes) => serde_json::from_slice(&bytes).map_err(Into::into),
			ws::Message::Ping(_) | ws::Message::Pong(_) => Err(DecodeMessageError::NotJson),
			ws::Message::Close(close_frame) => {
				Err(DecodeMessageError::ConnectionClosed { close_frame })
			}
		}
	}
}

/// A response to a [`Hello`] message.
///
/// This message is part of the initial handshake.
///
/// [`Hello`]: Incoming::Hello
#[derive(Debug, Serialize)]
pub struct HelloAck
{
	/// The interval at which the client should send heartbeat messages.
	pub heartbeat_interval: Seconds,
}

impl HelloAck
{
	/// Creates a new [`HelloAck`].
	pub fn new(heartbeat_interval: impl Into<Seconds>) -> Self
	{
		Self { heartbeat_interval: heartbeat_interval.into() }
	}

	/// Encodes a message so we can send it to a client.
	#[tracing::instrument(err(Debug))]
	pub fn encode(&self) -> Result<ws::Message, EncodeMessageError>
	{
		let json = serde_json::to_string(self)?;

		Ok(ws::Message::Text(json))
	}
}
