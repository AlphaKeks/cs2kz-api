use {
	crate::{
		checksum::Checksum,
		error::ResultExt,
		maps::{CourseLocalId, Map},
		players::{PlayerId, PlayerIp, PlayerName, PlayerPreferences},
		records::{self, CreatedRankedRecordData, RecordId},
	},
	serde::{Deserialize, Serialize, Serializer},
	std::error::Error,
	tokio_websockets::Message as RawMessage,
};

/// A WebSocket message ID
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct MessageId(u64);

impl MessageId
{
	pub(super) fn as_u64(&self) -> u64
	{
		self.0
	}
}

/// A WebSocket message
#[derive(Debug, Serialize)]
pub(super) struct Message<T>
{
	/// ID assigned by the client
	id: MessageId,

	/// The rest of the message payload
	#[serde(flatten)]
	payload: T,
}

/// Payloads for incoming [`Message`]s
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(super) enum Incoming
{
	/// The server changed map.
	MapChanged
	{
		/// The name of the new map
		name: Box<str>,
	},

	/// A player joined the server.
	PlayerJoin
	{
		/// The player's ID
		id: PlayerId,

		/// The player's name when they joined
		name: PlayerName,

		/// The player's IP address
		ip_address: PlayerIp,
	},

	/// A player left the server.
	PlayerLeave
	{
		/// The player's ID
		id: PlayerId,

		/// The player's name when they left
		name: PlayerName,

		/// The player's in-game preferences when they left
		preferences: PlayerPreferences,
	},

	/// A player is submitting a record.
	SubmitRecord
	{
		/// Local ID of the course the record was set on
		course_local_id: CourseLocalId,

		/// Checksum of the mode this record was set with
		mode_checksum: Checksum,

		/// ID of the player submitting the record
		player_id: PlayerId,

		/// The duration of the run
		time: records::Time,

		/// The number of teleports used
		teleports: records::Teleports,

		/// Checksums of the styles this record was set with
		style_checksums: Vec<Checksum>,
	},
}

/// Error for decoding [`Incoming`] messages
#[derive(Debug, Display, Error)]
pub(super) enum DecodeMessageError
{
	#[display("missing `id` field")]
	NoId
	{
		#[error(source)]
		error: serde_json::Error,
	},

	#[display("invalid payload: {error}")]
	InvalidPayload
	{
		id: MessageId,
		#[error(source)]
		error: serde_json::Error,
	},
}

/// Payloads for outgoing [`Message`]s
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub(super) enum Outgoing<'a>
{
	/// A generic error message
	Error
	{
		#[serde(serialize_with = "serialize_dyn_error")]
		error: &'a (dyn Error + Send + Sync),
	},

	/// The server should broadcast a message in chat.
	BroadcastMessage
	{
		/// The message to broadcast
		message: &'a str,
	},

	/// Response to [`Incoming::MapChanged`]
	MapChangedAck
	{
		/// Detailed information about the map
		map_info: Option<&'a Map>,
	},

	/// Response to [`Incoming::PlayerJoin`]
	PlayerJoinAck
	{
		/// The player's in-game preferences
		preferences: &'a PlayerPreferences,

		/// Whether the player is currently banned
		is_banned: bool,
	},

	/// Response to [`Incoming::SubmitRecord`]
	SubmitRecordAck
	{
		/// ID of the submitted record
		record_id: RecordId,

		/// Data about ranks, points, etc. if this record is a PB
		ranked_data: Option<&'a CreatedRankedRecordData>,
	},
}

/// Error for encoding [`Outgoing`] messages
#[derive(Debug, Display, Error, From)]
pub(super) struct EncodeMessageError(serde_json::Error);

impl<T> Message<T>
{
	pub(super) fn into_parts(self) -> (MessageId, T)
	{
		(self.id, self.payload)
	}
}

impl<T> Message<T>
where
	T: Serialize,
{
	/// Creates a new outgoing [`Message`] with an ID of 0.
	pub(super) fn new(payload: T) -> Self
	{
		Self { id: MessageId(0), payload }
	}

	/// Creates a new outgoing [`Message`] as a reply to an incoming message
	/// with ID `to`.
	pub(super) fn reply(to: MessageId, payload: T) -> Self
	{
		Self { id: to, payload }
	}

	/// Encodes `self` into a [`RawMessage`].
	pub(super) fn encode(&self) -> Result<RawMessage, EncodeMessageError>
	{
		serde_json::to_string(self)
			.map(RawMessage::text)
			.map_err(EncodeMessageError::from)
	}

	/// Encodes `self` into a [`RawMessage`], logging an error if the conversion
	/// failed.
	pub(super) fn encode_lossy(&self) -> Option<RawMessage>
	{
		self.encode()
			.inspect_err_dyn(|err| error!(error = err as &dyn Error, "failed to encode message"))
			.ok()
	}
}

impl<'de, T> Message<T>
where
	T: Deserialize<'de>,
{
	/// Decodes an incoming message payload.
	pub(super) fn decode(bytes: &'de [u8]) -> Result<Self, DecodeMessageError>
	{
		#[derive(Debug, Deserialize)]
		struct JustId
		{
			id: MessageId,
		}

		let JustId { id } =
			serde_json::from_slice(bytes).map_err(|err| DecodeMessageError::NoId { error: err })?;

		match serde_json::from_slice::<T>(bytes) {
			Ok(payload) => Ok(Self { id, payload }),
			Err(err) => Err(DecodeMessageError::InvalidPayload { id, error: err }),
		}
	}
}

impl<'a> From<&'a DecodeMessageError> for Message<Outgoing<'a>>
{
	fn from(error: &'a DecodeMessageError) -> Self
	{
		match *error {
			DecodeMessageError::NoId { ref error } => Message::new(Outgoing::Error { error }),
			DecodeMessageError::InvalidPayload { id, ref error } => {
				Message::reply(id, Outgoing::Error { error })
			},
		}
	}
}

fn serialize_dyn_error<S>(error: &dyn Error, serializer: S) -> Result<S::Ok, S::Error>
where
	S: Serializer,
{
	format_args!("{error}").serialize(serializer)
}
