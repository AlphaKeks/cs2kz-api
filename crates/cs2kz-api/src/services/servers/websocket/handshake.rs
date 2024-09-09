use axum::extract::ws;
use serde::{Deserialize, Serialize};

use super::message::{DecodeMessageError, EncodeMessageError};
use crate::services::plugin::PluginVersionName;

/// The message coming from the client initiating the handshake.
#[derive(Debug, Deserialize)]
pub struct Hello
{
	/// The cs2kz-metamod version the server is currently running.
	#[expect(dead_code, reason = "TODO: validate this")]
	plugin_version: PluginVersionName,

	/// The name of the map the server is currently hosting.
	#[expect(dead_code, reason = "TODO: fetch map and send info in `HelloAck`")]
	current_map: String,
}

impl Hello
{
	pub fn decode(raw: &ws::Message) -> Result<Self, DecodeMessageError>
	{
		match raw {
			ws::Message::Text(text) => serde_json::from_str(text)
				.map_err(|source| DecodeMessageError::InvalidJSON { id: None, source }),
			ws::Message::Binary(bytes) => serde_json::from_slice(bytes)
				.map_err(|source| DecodeMessageError::InvalidJSON { id: None, source }),
			ws::Message::Ping(_) | ws::Message::Pong(_) => Err(DecodeMessageError::NotJSON),
			ws::Message::Close(frame) => Err(DecodeMessageError::ConnectionClosed {
				frame: frame.as_ref().cloned(),
			}),
		}
	}
}

/// The reply from the server acknowledging the handshake.
#[derive(Debug, Serialize)]
pub struct HelloAck
{
	// FIXME
	foo: String,
}

impl HelloAck
{
	pub fn new(hello: &Hello) -> Self
	{
		debug!(?hello, "ACK'ing hello message");

		Self {
			foo: String::from("bar"),
		}
	}

	pub fn encode(&self) -> Result<ws::Message, EncodeMessageError>
	{
		serde_json::to_string(self)
			.map(ws::Message::Text)
			.map_err(Into::into)
	}
}
