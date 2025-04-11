pub use self::config::Config;
use {
	crate::{
		database,
		event_queue::{self, Event},
		points::PointsDaemonHandle,
		servers::{self, ServerId},
	},
	axum_tws::{WebSocketError, WebSocketUpgrade},
	futures_util::{FutureExt, StreamExt, TryStreamExt, future, stream},
	std::{
		assert_matches::debug_assert_matches,
		collections::hash_map::{self, HashMap},
		error::Error,
		fmt,
		sync::Arc,
	},
	tokio::{
		sync::{mpsc, oneshot},
		task::{self, JoinSet},
	},
	tokio_util::sync::CancellationToken,
};

mod config;
mod proto;

pub type Result<T, E = ServerMonitorError> = std::result::Result<T, E>;
type TaskResult = Result<(), WebSocketError>;
type HttpResponse = axum::response::Response;

pub struct ServerMonitor
{
	config: Config,
	database: database::ConnectionPool,
	points_daemon: PointsDaemonHandle,

	tasks: JoinSet<TaskResult>,
	server_ids: HashMap<task::Id, ServerId>,
	server_data: HashMap<ServerId, ConnectedServerData>,

	handles_tx: mpsc::Sender<HandleMessage>,
	handles_rx: mpsc::Receiver<HandleMessage>,
}

#[derive(Debug, Clone)]
pub struct ServerMonitorHandle
{
	monitor_tx: mpsc::WeakSender<HandleMessage>,
}

#[derive(Debug, Display, Error)]
pub enum ServerMonitorError {}

#[derive(Debug, Display, Error)]
pub enum ServerConnectingError
{
	#[display("monitor is currently unavailable")]
	MonitorUnavailable,

	#[display("server is already connected")]
	AlreadyConnected,
}

#[derive(Debug, Display, Error)]
pub enum DisconnectServerError
{
	#[display("monitor is currently unavailable")]
	MonitorUnavailable,
}

#[derive(Debug, Display, Error)]
pub enum GetConnectionInfoError
{
	#[display("server is not currently connected")]
	NotConnected,

	#[display("monitor is currently unavailable")]
	MonitorUnavailable,
}

#[derive(Debug, Display, Error)]
pub enum BroadcastMessageError
{
	#[display("monitor is currently unavailable")]
	MonitorUnavailable,
}

#[derive(Debug)]
struct ConnectedServerData
{
	tx: mpsc::Sender<ServerMessage>,
	abort_handle: task::AbortHandle,
}

#[derive(Debug)]
enum HandleMessage
{
	ServerConnecting
	{
		/// ID of the server that is connecting
		id: ServerId,

		/// HTTP upgrade extracted from the request
		http_upgrade: WebSocketUpgrade,

		/// Either the HTTP response to return or an error if the server could
		/// not connect
		response_tx: oneshot::Sender<Result<HttpResponse, ServerConnectingError>>,
	},

	DisconnectServer
	{
		/// The ID of the server to disconnect
		id: ServerId,

		/// Whether the server was disconnected
		response_tx: oneshot::Sender<bool>,
	},

	WantConnectionInfo
	{
		/// The ID of the server we want information from
		id: ServerId,

		/// Information about the connection
		response_tx: oneshot::Sender<Option<servers::ConnectionInfo>>,
	},

	BroadcastMessage
	{
		/// ID of the server that should broadcast the message
		server_id: ServerId,

		/// The message to be broadcast
		message: Arc<str>,

		/// Signal to confirm the message was (not) broadcast
		response_tx: oneshot::Sender<bool>,
	},
}

#[derive(Debug)]
enum ServerMessage
{
	Disconnect
	{
		response_tx: oneshot::Sender<bool>
	},

	WantConnectionInfo
	{
		response_tx: oneshot::Sender<Option<servers::ConnectionInfo>>
	},

	BroadcastMessage
	{
		message: Arc<str>, response_tx: oneshot::Sender<bool>
	},
}

impl ServerMonitor
{
	pub fn new(
		config: Config,
		database: database::ConnectionPool,
		points_daemon: PointsDaemonHandle,
	) -> Self
	{
		let tasks = JoinSet::default();
		let server_ids = HashMap::default();
		let server_data = HashMap::default();
		let (handles_tx, handles_rx) = mpsc::channel(128);

		Self {
			config,
			database,
			points_daemon,
			tasks,
			server_ids,
			server_data,
			handles_tx,
			handles_rx,
		}
	}

	pub fn handle(&self) -> ServerMonitorHandle
	{
		ServerMonitorHandle { monitor_tx: self.handles_tx.downgrade() }
	}

	#[instrument(skip(cancellation_token), err)]
	pub async fn run(mut self, cancellation_token: CancellationToken) -> Result<()>
	{
		loop {
			select! {
				() = cancellation_token.cancelled() => {
					info!("server monitor shutting down");
					return Ok(());
				},

				Some(join_result) = self.tasks.join_next_with_id() => {
					self.on_task_complete(join_result).await;
				},

				Some(message) = self.handles_rx.recv() => {
					self.on_handle_message(message).await;
				},
			};
		}
	}

	#[instrument]
	async fn on_task_complete(
		&mut self,
		join_result: Result<(task::Id, TaskResult), task::JoinError>,
	)
	{
		match join_result {
			Ok((task_id, task_result)) => {
				let Some(server_id) = self.server_ids.remove(&task_id) else {
					unreachable!()
				};

				let Some(data) = self.server_data.remove(&server_id) else {
					unreachable!()
				};

				debug_assert!(data.tx.is_closed());
				debug_assert_eq!(data.abort_handle.id(), task_id);
				debug_assert!(data.abort_handle.is_finished());

				if let Err(err) = task_result {
					warn!(
						task.id = %task_id,
						server.id = %server_id,
						error = &err as &dyn Error,
						"server task encountered an error",
					);
				} else {
					info!(task.id = %task_id, server.id = %server_id, "server disconnected");
				}

				event_queue::dispatch(Event::ServerDisconnected { id: server_id });
			},
			Err(err) => {
				error!(error = &err as &dyn Error, "failed to join server task");
			},
		}
	}

	#[instrument]
	async fn on_handle_message(&mut self, message: HandleMessage)
	{
		match message {
			HandleMessage::ServerConnecting { id, http_upgrade, response_tx } => {
				let _ = response_tx.send(self.on_server_connecting(id, http_upgrade).await);
			},
			HandleMessage::DisconnectServer { id, response_tx } => {
				self.disconnect_server(id, response_tx).await;
			},
			HandleMessage::WantConnectionInfo { id, response_tx } => {
				self.get_connection_info(id, response_tx).await;
			},
			HandleMessage::BroadcastMessage { server_id, message, response_tx } => {
				self.broadcast_message(server_id, message, response_tx).await;
			},
		}
	}

	#[instrument]
	async fn on_server_connecting(
		&mut self,
		id: ServerId,
		http_upgrade: WebSocketUpgrade,
	) -> Result<HttpResponse, ServerConnectingError>
	{
		match self.server_data.entry(id) {
			hash_map::Entry::Occupied(entry) => {
				debug_assert!(self.server_ids.contains_key(&entry.get().abort_handle.id()));
				Err(ServerConnectingError::AlreadyConnected)
			},

			hash_map::Entry::Vacant(entry) => {
				if cfg!(debug_assertions) {
					self.server_ids.values().for_each(|&server_id| assert_ne!(server_id, id));
				}

				// Because `on_upgrade` (see end of this function) spawns a task
				// internally, but we want to spawn tasks ourselves (in
				// `self.tasks`), we have to set up a channel for sending the
				// socket from the task spawned by `on_upgrade` to our own task.
				let (socket_tx, socket_rx) = oneshot::channel();
				let (server_tx, server_rx) = mpsc::channel(32);
				let config = self.config;
				let database = self.database.clone();
				let points_daemon = self.points_daemon.clone();
				let server_data = entry.insert(ConnectedServerData {
					tx: server_tx,
					abort_handle: self
						.tasks
						.build_task()
						.name(&format!("gameserver_{id}"))
						.spawn(socket_rx.then(move |socket_result| {
							match socket_result {
								Ok(socket) => future::Either::Left({
									proto::serve_connection(
										socket,
										server_rx,
										config,
										database,
										points_daemon,
										id,
									)
								}),
								Err(_) => future::Either::Right(if cfg!(debug_assertions) {
									future::ready(Ok((/* debug assertion below failed */)))
								} else {
									unreachable!("`socket_tx` is not dropped")
								}),
							}
						}))
						.unwrap_or_else(|err| panic!("failed to spawn task: {err}")),
				});

				{
					let old_id = self.server_ids.insert(server_data.abort_handle.id(), id);
					debug_assert_matches!(old_id, None);
				}

				Ok(http_upgrade.on_upgrade(async move |socket| {
					let _ = socket_tx.send(socket);
				}))
			},
		}
	}

	#[instrument]
	async fn disconnect_server(&mut self, id: ServerId, response_tx: oneshot::Sender<bool>)
	{
		let Some(&ConnectedServerData { ref tx, .. }) = self.server_data.get(&id) else {
			let _ = response_tx.send(false);
			return;
		};

		if let Err(mpsc::error::SendError(message)) =
			tx.send(ServerMessage::Disconnect { response_tx }).await
		{
			let ServerMessage::Disconnect { response_tx } = message else {
				unreachable!()
			};

			let _ = response_tx.send(false);
		}
	}

	#[instrument]
	async fn get_connection_info(
		&mut self,
		id: ServerId,
		response_tx: oneshot::Sender<Option<servers::ConnectionInfo>>,
	)
	{
		let Some(&ConnectedServerData { ref tx, .. }) = self.server_data.get(&id) else {
			let _ = response_tx.send(None);
			return;
		};

		if let Err(mpsc::error::SendError(message)) =
			tx.send(ServerMessage::WantConnectionInfo { response_tx }).await
		{
			let ServerMessage::WantConnectionInfo { response_tx } = message else {
				unreachable!()
			};

			let _ = response_tx.send(None);
		}
	}

	#[instrument]
	async fn broadcast_message(
		&mut self,
		server_id: ServerId,
		message: Arc<str>,
		response_tx: oneshot::Sender<bool>,
	)
	{
		let Some(&ConnectedServerData { ref tx, .. }) = self.server_data.get(&server_id) else {
			let _ = response_tx.send(false);
			return;
		};

		if let Err(mpsc::error::SendError(message)) =
			tx.send(ServerMessage::BroadcastMessage { message, response_tx }).await
		{
			let ServerMessage::BroadcastMessage { response_tx, .. } = message else {
				unreachable!()
			};

			let _ = response_tx.send(false);
		}
	}
}

#[expect(clippy::missing_fields_in_debug)]
impl fmt::Debug for ServerMonitor
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_struct("ServerMonitor")
			.field("config", &self.config)
			.field("tasks", &self.tasks.len())
			.field("handles", &self.handles_tx.weak_count())
			.finish()
	}
}

impl ServerMonitorHandle
{
	pub fn dangling() -> Self
	{
		Self { monitor_tx: mpsc::channel(1).0.downgrade() }
	}

	#[instrument(skip(self, http_upgrade), err(level = "debug"))]
	pub async fn server_connecting(
		&self,
		id: ServerId,
		http_upgrade: WebSocketUpgrade,
	) -> Result<HttpResponse, ServerConnectingError>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let tx = self
			.monitor_tx
			.upgrade()
			.ok_or(ServerConnectingError::MonitorUnavailable)?;

		if tx
			.send(HandleMessage::ServerConnecting { id, http_upgrade, response_tx })
			.await
			.is_err()
		{
			return Err(ServerConnectingError::MonitorUnavailable);
		}

		response_rx.await.unwrap_or(Err(ServerConnectingError::MonitorUnavailable))
	}

	#[instrument(skip(self), ret(level = "debug"), err(level = "debug"))]
	pub async fn disconnect_server(&self, id: ServerId) -> Result<bool, DisconnectServerError>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let tx = self
			.monitor_tx
			.upgrade()
			.ok_or(DisconnectServerError::MonitorUnavailable)?;

		if tx.send(HandleMessage::DisconnectServer { id, response_tx }).await.is_err() {
			return Err(DisconnectServerError::MonitorUnavailable);
		}

		response_rx.await.map_err(|_| DisconnectServerError::MonitorUnavailable)
	}

	#[instrument(skip(self), ret(level = "debug"), err(level = "debug"))]
	pub async fn get_connection_info(
		&self,
		id: ServerId,
	) -> Result<servers::ConnectionInfo, GetConnectionInfoError>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let tx = self
			.monitor_tx
			.upgrade()
			.ok_or(GetConnectionInfoError::MonitorUnavailable)?;

		if tx
			.send(HandleMessage::WantConnectionInfo { id, response_tx })
			.await
			.is_err()
		{
			return Err(GetConnectionInfoError::MonitorUnavailable);
		}

		response_rx
			.await
			.map_err(|_| GetConnectionInfoError::MonitorUnavailable)?
			.ok_or(GetConnectionInfoError::NotConnected)
	}

	#[instrument(skip_all, ret(level = "debug"), err(level = "debug"))]
	pub async fn broadcast_message(
		&self,
		to: impl IntoIterator<Item = ServerId>,
		message: impl Into<Arc<str>>,
	) -> Result<usize, BroadcastMessageError>
	{
		let tx = self
			.monitor_tx
			.upgrade()
			.ok_or(BroadcastMessageError::MonitorUnavailable)?;

		let message = Into::<Arc<str>>::into(message);

		let responses = stream::iter(to).then(|server_id| {
			let (response_tx, response_rx) = oneshot::channel();
			let message = HandleMessage::BroadcastMessage {
				server_id,
				message: Arc::clone(&message),
				response_tx,
			};

			tx.send(message).then(async move |send_result| {
				send_result.map_err(|_| BroadcastMessageError::MonitorUnavailable)?;
				response_rx.await.map_err(|_| BroadcastMessageError::MonitorUnavailable)
			})
		});

		responses
			.try_fold(0_usize, async |count, received| Ok(count + usize::from(received)))
			.await
	}
}
