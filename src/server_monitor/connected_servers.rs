use std::{
	collections::hash_map::{self, HashMap},
	error::Error,
	fmt,
	io,
	pin::pin,
};

use axum::response::Response;
use axum_tws::{WebSocket, WebSocketError, WebSocketUpgrade};
use futures_util::{SinkExt, StreamExt, TryStreamExt, stream};
use tokio::{
	sync::{mpsc, oneshot},
	task::{self, JoinSet},
};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;

use super::{ServerMessage, ServerMonitorConfig, proto};
use crate::{database::Database, error::ResultExt, servers::ServerId};

/// A collection of gameserver WebSocket tasks
#[derive(Default)]
pub(super) struct ConnectedServers
{
	/// The [`JoinSet`] we spawn our tasks onto
	join_set: JoinSet<()>,

	/// Extra data stored for each task
	task_data: HashMap<ServerId, ServerTaskData>,

	/// Mapping from task ID to server ID so we can perform a reverse lookup
	/// into `task_data` to cancel tasks
	servers_by_task: HashMap<task::Id, ServerId>,
}

#[derive(Debug)]
pub(super) struct ServerTaskData
{
	/// The server's ID
	pub id: ServerId,

	/// The task handle we received from [`JoinSet`]
	pub abort_handle: task::AbortHandle,

	/// A sender to communicate with the task
	pub tx: mpsc::WeakSender<ServerMessage>,
}

#[derive(Debug, Display, Error)]
#[display("failed to send message to server: {_variant}")]
pub(super) enum SendServerMessageError
{
	#[display("server is not currently connected")]
	#[error(ignore)]
	ServerNotConnected
	{
		message: ServerMessage
	},
}

impl ConnectedServers
{
	/// Inserts a new server and spawns a task for it.
	///
	/// In case the server is already connected, the task ID of its task is
	/// returned.
	#[tracing::instrument(
		level = "debug",
		skip(self, http_upgrade, database, cancellation_token),
		err(level = "debug")
	)]
	pub(super) fn insert(
		&mut self,
		server_id: ServerId,
		http_upgrade: WebSocketUpgrade,
		config: ServerMonitorConfig,
		database: Database,
		cancellation_token: CancellationToken,
	) -> Result<Response, task::Id>
	{
		let entry = match self.task_data.entry(server_id) {
			hash_map::Entry::Vacant(entry) => entry,
			hash_map::Entry::Occupied(entry) => {
				return Err(entry.get().abort_handle.id());
			},
		};

		let (orig_tx, rx) = mpsc::channel(16);
		let tx = orig_tx.downgrade();

		let (socket_tx, socket_rx) = oneshot::channel::<WebSocket>();
		let future = async move {
			if let Ok(socket) = socket_rx.await {
				let map_err = |err| {
					if let WebSocketError::Internal(axum_tws::Error::Io(error)) = err {
						error
					} else {
						io::Error::other(err)
					}
				};

				let mut socket = pin!(socket.sink_map_err(map_err).map_err(map_err));

				if let Err(err) = proto::main_loop(
					socket.as_mut(),
					server_id,
					database,
					cancellation_token,
					config,
					rx,
				)
				.await
				{
					tracing::error!(error = &err as &dyn Error, "server encountered an error");

					if let Err(err) = socket.send(proto::internal_server_error()).await {
						tracing::error!(error = &err as &dyn Error, "failed to send close frame",);
					}
				}

				tracing::debug!("server disconnected");

				drop(orig_tx);
			}
		}
		.instrument({
			tracing::info_span!("gameserver", id = %server_id, connection_id = %Uuid::now_v7())
		});

		let abort_handle = self
			.join_set
			.build_task()
			.name(&format!("gameserver_{server_id}"))
			.spawn(future)
			.unwrap_or_else(|err| panic!("failed to spawn task: {err}"));

		tracing::trace!("spawned game server task");

		self.servers_by_task.insert(abort_handle.id(), server_id);
		entry.insert(ServerTaskData { id: server_id, abort_handle, tx });

		tracing::trace!("stored game server task");

		Ok(http_upgrade.on_upgrade(async move |socket| {
			tracing::trace!("upgraded HTTP connection to WebSocket");
			let _ = socket_tx.send(socket);
		}))
	}

	/// Waits for the next task to finish and returns its data.
	#[tracing::instrument(level = "trace", skip(self), ret(level = "trace"))]
	pub(super) async fn join_next(&mut self) -> Option<ServerTaskData>
	{
		let task_result = self.join_set.join_next_with_id().await?;
		let (task_id, ()) = task_result
			.inspect_err_dyn(|error| tracing::error!(error, "failed to join server task"))
			.ok()?;

		self.servers_by_task.remove(&task_id).and_then(|server_id| {
			let task_data = self.task_data.remove(&server_id);

			if task_data.is_none() {
				tracing::warn!("lost task data for server {server_id}?");
			}

			task_data
		})
	}

	/// Sends a message to a specific server.
	#[tracing::instrument(level = "debug", skip(self), err(level = "debug"))]
	pub(super) async fn send_message(
		&self,
		server_id: ServerId,
		message: ServerMessage,
	) -> Result<(), SendServerMessageError>
	{
		let Some(tx) = self.task_data.get(&server_id).and_then(|task_data| task_data.tx.upgrade())
		else {
			return Err(SendServerMessageError::ServerNotConnected { message });
		};

		tx.send(message)
			.await
			.map_err(|err| SendServerMessageError::ServerNotConnected { message: err.0 })
	}

	/// Broadcasts a message to all connected servers.
	///
	/// Returns the number of servers the message was sent to.
	#[tracing::instrument(level = "debug", skip(self))]
	pub(super) fn broadcast_message(&self, message: &ServerMessage) -> impl Future<Output = usize>
	{
		stream::iter(self.task_data.iter()).fold(0, async |count, (&server_id, task_data)| {
			let Some(tx) = task_data.tx.upgrade() else {
				tracing::warn!("server {server_id} unavailable but still in `ConnectedServers`");
				return count;
			};

			if let Err(_) = tx.send(message.clone()).await {
				tracing::warn!("server {server_id} unavailable but still in `ConnectedServers`");
				return count;
			}

			count + 1
		})
	}
}

impl fmt::Debug for ConnectedServers
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_struct("ConnectedServers")
			.field("count", &self.join_set.len())
			.finish_non_exhaustive()
	}
}
