use std::sync::{Arc, LazyLock};

use axum::response::sse;
use futures_util::{FutureExt, Stream, stream};
use serde::Serialize;
use tokio::sync::broadcast;
use utoipa::ToSchema;

use crate::{
	maps::{MapId, MapName},
	players::PlayerId,
	records::RecordId,
	servers::{self, ConnectedPlayerInfo, ServerId},
};

static QUEUE: LazyLock<broadcast::Sender<Arc<Event>>> = LazyLock::new(|| broadcast::channel(32).0);

#[non_exhaustive]
#[derive(Debug, Serialize, ToSchema)]
#[serde(untagged)]
pub enum Event
{
	/// `lag` - You missed events because you consumed them too slowly.
	Lag
	{
		/// The number of events that were skipped
		skipped: u64,
	},

	/// `map-created` - A new map has been submitted.
	MapCreated
	{
		/// The ID of the map
		id: MapId,

		/// The name of the map
		name: MapName,
	},

	/// `map-approved` - A map has been accepted into the global map pool.
	MapApproved
	{
		/// The ID of the map
		id: MapId,
	},

	/// `server-connected` - A server has connected to the API.
	ServerConnected(servers::ConnectionInfo),

	/// `server-disconnected` - A server has disconnected from the API.
	ServerDisconnected
	{
		/// The ID of the server
		id: ServerId,
	},

	/// `player-join` - A player joined a server.
	PlayerJoin
	{
		/// The ID of the server the player joined
		server_id: ServerId,

		/// The player that joined
		player: ConnectedPlayerInfo,
	},

	/// `player-leave` - A player left a server.
	PlayerLeave
	{
		/// The ID of the server the player left
		server_id: ServerId,

		/// The ID of the player that left
		player_id: PlayerId,
	},

	/// `record-submitted` - A new record has been submitted.
	RecordSubmitted
	{
		/// The ID of the record
		record_id: RecordId,
	},
}

impl TryFrom<&Event> for sse::Event
{
	type Error = axum::Error;

	fn try_from(event: &Event) -> Result<Self, Self::Error>
	{
		let name = match *event {
			Event::Lag { .. } => "lag",
			Event::MapCreated { .. } => "map-created",
			Event::MapApproved { .. } => "map-approved",
			Event::ServerConnected(..) => "server-connected",
			Event::ServerDisconnected { .. } => "server-disconnected",
			Event::PlayerJoin { .. } => "player-join",
			Event::PlayerLeave { .. } => "player-leave",
			Event::RecordSubmitted { .. } => "record-submitted",
		};

		sse::Event::default().event(name).json_data(event)
	}
}

/// Returns a [`Stream`] of [`Event`]s.
pub fn subscribe() -> impl Stream<Item = Arc<Event>>
{
	stream::unfold(QUEUE.subscribe(), async |mut rx| {
		let item = rx.recv().map(|recv_result| match recv_result {
			Ok(event) => Some(event),
			Err(broadcast::error::RecvError::Lagged(n)) => {
				Some(Arc::new(Event::Lag { skipped: n }))
			},
			Err(broadcast::error::RecvError::Closed) => None,
		});

		item.await.map(|item| (item, rx))
	})
}

#[tracing::instrument(ret(level = "debug"))]
pub(crate) fn dispatch(event: Event) -> usize
{
	QUEUE.send(Arc::new(event)).unwrap_or_else(|_| {
		tracing::trace!("no active event listeners");
		0_usize
	})
}
