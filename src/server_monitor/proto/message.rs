use std::{error::Error, fmt};

use serde::{Deserialize, Serialize};
use tokio_websockets::proto::Message as RawMessage;

use crate::{
	checksum::Checksum,
	maps::{CourseId, Map},
	players::{PlayerId, PlayerIp, PlayerName, PlayerPreferences},
	records::{CreatedRankedRecordData, RecordId, Teleports, Time},
};

/// A message sent between client and server
#[derive(Debug, Serialize, Deserialize)]
pub(in crate::server_monitor) struct Message<T>
{
	/// ID assigned by the client
	id: u32,

	/// The rest of the message
	#[serde(flatten)]
	payload: T,
}

/// Messages we expect to receive from the client
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(in crate::server_monitor) enum Incoming
{
	/// The server has changed maps.
	MapChange
	{
		name: Box<str>
	},

	/// A player has joined the server.
	PlayerJoin
	{
		id: PlayerId,
		name: PlayerName,
		ip_address: PlayerIp,
	},

	/// A player has left the server.
	PlayerLeave
	{
		id: PlayerId,
		name: PlayerName,
		preferences: PlayerPreferences,
	},

	/// A player wants to submit a record.
	NewRecord
	{
		player_id: PlayerId,
		course_id: CourseId,
		mode_checksum: Checksum,
		style_checksums: Vec<Checksum>,
		time: Time,
		teleports: Teleports,
	},
}

#[derive(Debug, Display, Error)]
#[display("failed to decode message: {kind}")]
pub(in crate::server_monitor) struct DecodeMessageError
{
	kind: DecodeMessageErrorKind,
	source: serde_json::Error,
}

#[derive(Debug, Display)]
pub(in crate::server_monitor) enum DecodeMessageErrorKind
{
	#[display("missing `id` field")]
	MissingId,

	#[display("failed to deserialize payload")]
	DeserializePayload
	{
		message_id: u32
	},
}

/// Messages we intend to send to the client
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(in crate::server_monitor) enum Outgoing<'a>
{
	/// A generic error message
	Error
	{
		#[serde(rename = "error")]
		message: Box<str>,
	},

	/// The server should broadcast a message in chat
	BroadcastChatMessage
	{
		message: &'a str
	},

	/// Response to [`Incoming::MapChange`]
	MapChangeAck
	{
		map: Option<&'a Map>
	},

	/// Response to [`Incoming::PlayerJoin`]
	PlayerJoinAck
	{
		player_id: PlayerId,
		is_banned: bool,
		preferences: &'a PlayerPreferences,
	},

	/// Response to [`Incoming::NewRecord`]
	NewRecordAck
	{
		record_id: RecordId,
		ranked_data: Option<&'a CreatedRankedRecordData>,
	},
}

#[derive(Debug, Display, Error, From)]
#[display("failed to encode message: {_0}")]
pub(in crate::server_monitor) struct EncodeMessageError(serde_json::Error);

impl<T> Message<T>
{
	pub(in crate::server_monitor) fn into_parts(self) -> (u32, T)
	{
		(self.id, self.payload)
	}
}

impl<T> Message<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug,
{
	/// Decodes an incoming message.
	#[track_caller]
	#[tracing::instrument(level = "trace", ret(level = "trace"), err(level = "debug"))]
	pub(in crate::server_monitor) fn decode(raw: &RawMessage) -> Result<Self, DecodeMessageError>
	{
		#[derive(Debug, Deserialize)]
		struct JustId
		{
			id: u32,
		}

		debug_assert!(raw.is_binary() || raw.is_text());

		let payload = raw.as_payload();
		let JustId { id } = serde_json::from_slice(&payload[..]).map_err(|err| {
			DecodeMessageError { kind: DecodeMessageErrorKind::MissingId, source: err }
		})?;

		serde_json::from_slice(&payload[..])
			.map(|payload| Self { id, payload })
			.map_err(|err| DecodeMessageError {
				kind: DecodeMessageErrorKind::DeserializePayload { message_id: id },
				source: err,
			})
	}
}

impl<T> Message<T>
where
	T: Serialize + fmt::Debug,
{
	/// Creates a new outgoing message.
	///
	/// If this is a reply, `message_id` should be the same as the incoming
	/// message's. Otherwise it should be `0`.
	pub(in crate::server_monitor) fn new(message_id: u32, payload: T) -> Self
	{
		Self { id: message_id, payload }
	}

	/// Encodes an outgoing message so it can be sent over a WebSocket.
	#[tracing::instrument(level = "trace", ret(level = "trace"), err)]
	pub(in crate::server_monitor) fn encode(&self) -> Result<RawMessage, EncodeMessageError>
	{
		serde_json::to_string(self)
			.map(RawMessage::text)
			.map_err(EncodeMessageError::from)
	}
}

impl Message<Outgoing<'_>>
{
	/// Creates a new outgoing error message.
	///
	/// This should be used for errors rather than [`Message::new()`] because it
	/// will also capture the original error value in our logs.
	#[tracing::instrument(level = "debug", ret(level = "trace"))]
	pub(in crate::server_monitor) fn error(message_id: u32, error: &(dyn Error + 'static)) -> Self
	{
		Self {
			id: message_id,
			payload: Outgoing::Error { message: error.to_string().into_boxed_str() },
		}
	}
}
