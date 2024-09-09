use axum::extract::ws;
use serde::{Deserialize, Serialize};

use crate::services::players::PlayerInfo;

mod errors;
pub use errors::{DecodeMessageError, EncodeMessageError};

/// A WebSocket message with payload type `T`.
///
/// Every message holds both an ID and a JSON payload. The ID is used purely by the client to
/// connect response messages coming from the server to requests it has sent earlier, which means
/// we just echo it back as-is.
#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T>
{
	id: u64,
	payload: T,
}

/// Payloads for incoming messages.
#[derive(Debug, Deserialize)]
pub enum Incoming
{
	Heartbeat
	{
		/// List of currently online players.
		players: Vec<PlayerInfo>,
	},
}

/// Payloads for outgoing messages.
#[derive(Debug, Serialize)]
pub enum Outgoing
{
	Error
	{
		message: String
	},
}

impl<T> Message<T>
{
	/// Returns the message ID.
	pub const fn id(&self) -> u64
	{
		self.id
	}

	/// Returns a reference to the message payload.
	pub const fn payload(&self) -> &T
	{
		&self.payload
	}
}

impl Message<Incoming>
{
	/// Decodes an incoming message.
	pub fn decode(raw: &ws::Message) -> Result<Self, DecodeMessageError>
	{
		#[derive(Deserialize)]
		struct RawMessage
		{
			id: u64,
			payload: serde_json::Value,
		}

		let raw = match raw {
			ws::Message::Text(text) => serde_json::from_str::<RawMessage>(text)
				.map_err(|source| DecodeMessageError::InvalidJSON { id: None, source }),
			ws::Message::Binary(bytes) => serde_json::from_slice::<RawMessage>(bytes)
				.map_err(|source| DecodeMessageError::InvalidJSON { id: None, source }),
			ws::Message::Ping(_) | ws::Message::Pong(_) => Err(DecodeMessageError::NotJSON),
			ws::Message::Close(frame) => Err(DecodeMessageError::ConnectionClosed {
				frame: frame.as_ref().cloned(),
			}),
		}?;

		serde_json::from_value(raw.payload).map_err(|source| DecodeMessageError::InvalidJSON {
			id: Some(raw.id),
			source,
		})
	}
}

impl Message<Outgoing>
{
	/// Constructs an error message to send to the client.
	///
	/// Error messages will default to an ID of 0 if `message_id` is [`None`].
	pub fn error(error: &impl std::error::Error, message_id: Option<u64>) -> Self
	{
		Self {
			id: message_id.unwrap_or_default(),
			payload: Outgoing::Error {
				message: error.to_string(),
			},
		}
	}

	/// Constructs a new outgoing message as a reply to an incoming message.
	pub fn reply_to(to: &Message<Incoming>, payload: Outgoing) -> Self
	{
		Self {
			id: to.id(),
			payload,
		}
	}

	/// Encodes an outgoing message.
	pub fn encode(&self) -> Result<ws::Message, EncodeMessageError>
	{
		serde_json::to_string(self)
			.map(ws::Message::Text)
			.map_err(Into::into)
	}
}
