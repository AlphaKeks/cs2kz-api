//! WebSocket messages.

use axum::extract::ws;
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::PlayerInfo;
use crate::services::maps::FetchMapResponse;
use crate::services::players::Session;
use crate::util::MapIdentifier;

/// A WebSocket message.
///
/// Every message has an ID. This ID is set by the client and echoed back as-is.
/// It purely exists so clients can tie messages from us back to requests they
/// sent earlier.
///
/// The payload is different depending on whether we are receiving or sending
/// messages, so the type is split up into [`Incoming`] and [`Outgoing`]
/// messages.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T>
{
	/// The message ID.
	pub id: u64,

	/// The JSON payload.
	#[serde(flatten)]
	pub payload: T,
}

/// Messages we receive.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum Incoming
{
	/// The server changed map.
	MapChange
	{
		/// The name of the new map.
		map_name: String,
	},

	/// The player count changed.
	///
	/// This can happen if a player (dis)connects or if bots (de)spawn.
	PlayerCountChange
	{
		/// List of players authenticated with Steam.
		authenticated_players: Vec<PlayerInfo>,

		/// The total amount of online players (may include bots for example).
		total_players: u64,

		/// The amount of player slots on the server.
		max_players: u64,
	},

	/// Update for a player.
	PlayerUpdate
	{
		/// The player.
		player: PlayerInfo,

		/// The player's in-game preferences.
		preferences: serde_json::Value,

		/// Information about this session.
		session: Session,
	},

	/// Request to get a player's preferences.
	GetPreferences
	{
		/// The SteamID of the player whose preferences you want to get.
		player_id: SteamID,
	},

	/// The server wants information about a map.
	GetMap
	{
		/// The map's ID or name.
		map_identifier: MapIdentifier,
	},
}

/// Messages we send.
#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "payload", rename_all = "kebab-case")]
pub enum Outgoing
{
	/// Report an error to the client.
	Error
	{
		/// The error message.
		message: String,
	},

	/// Information about a map.
	///
	/// This is requested e.g. when the server switches map, or when a player
	/// uses the `!map` command.
	MapInfo(#[serde(serialize_with = "skip_option")] Option<FetchMapResponse>),

	/// A player's in-game preferences.
	Preferences
	{
		/// The preferences as key-value pairs.
		preferences: Option<serde_json::Map<String, serde_json::Value>>,
	},
}

pub fn skip_option<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
	T: serde::Serialize,
	S: serde::Serializer,
{
	match *value {
		None => serializer.serialize_none(),
		Some(ref value) => value.serialize(serializer),
	}
}

impl Message<Incoming>
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

impl Message<Outgoing>
{
	/// Creates an error message to be sent to the client.
	#[tracing::instrument]
	pub fn error<E>(error: &E) -> Self
	where
		E: std::error::Error + ?Sized,
	{
		Self { id: 0, payload: Outgoing::Error { message: error.to_string() } }
	}

	/// Encodes a message so we can send it to a client.
	#[tracing::instrument(ret(level = "debug"), err(Debug, level = "debug"))]
	pub fn encode(&self) -> Result<ws::Message, EncodeMessageError>
	{
		let json = serde_json::to_string(self)?;

		Ok(ws::Message::Text(json))
	}
}

/// Errors that can occur when decoding incoming messages.
#[derive(Debug, Error)]
pub enum DecodeMessageError
{
	/// We failed to parse the message payload as JSON.
	#[error(transparent)]
	ParseJson(#[from] serde_json::Error),

	/// The message payload was not text/binary.
	#[error("payload is not json")]
	NotJson,

	/// The message was a close message.
	#[error("client closed connection unexpectedly")]
	ConnectionClosed
	{
		/// The close frame that was included in the message, if any.
		close_frame: Option<ws::CloseFrame<'static>>,
	},
}

/// Errors that can occur when encoding an outgoing message.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct EncodeMessageError(#[from] serde_json::Error);
