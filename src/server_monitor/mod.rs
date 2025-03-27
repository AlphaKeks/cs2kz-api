mod config;
mod connected_servers;
mod proto;

use std::{future, sync::Arc};

use axum::response::Response;
use axum_tws::WebSocketUpgrade;
use futures_util::{StreamExt, stream};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;

pub use self::config::ServerMonitorConfig;
use self::connected_servers::{ConnectedServers, SendServerMessageError, ServerTaskData};
use crate::{
	database::{Database, DatabaseError},
	event_queue::{self, Event},
	servers::{self, ServerId},
};

/// A background task for managing WebSocket connections to gameservers.
#[derive(Debug)]
pub struct ServerMonitor
{
	/// The original sender half of the channel
	tx: mpsc::Sender<MonitorMessage>,

	/// The receiver we use to handle messages from [`ServerMonitorHandle`]
	rx: mpsc::Receiver<MonitorMessage>,

	database: Database,
	config: ServerMonitorConfig,

	/// A collection of gameserver WebSocket tasks
	connected_servers: ConnectedServers,
}

/// A handle to communicate with the [`ServerMonitor`]
#[derive(Debug, Clone)]
pub struct ServerMonitorHandle
{
	tx: mpsc::WeakSender<MonitorMessage>,
}

#[derive(Debug, Display, Error)]
pub enum ServerMonitorError
{
	#[display("{_0}")]
	Database(DatabaseError),
}

#[derive(Debug, Display, Error)]
pub enum ServerConnectingError
{
	#[display("monitor is unavailable")]
	MonitorUnavailable,

	#[display("server is already connected")]
	ServerAlreadyConnected,
}

#[derive(Debug, Display, Error)]
pub enum DisconnectServerError
{
	#[display("monitor is unavailable")]
	MonitorUnavailable,
}

#[derive(Debug, Display, Error)]
#[display("failed to get connection info: {_variant}")]
pub enum GetConnectionInfoError
{
	#[display("monitor is unavailable")]
	MonitorUnavailable,
}

#[derive(Debug, Display, Error)]
#[display("failed to broadcast chat message: {_variant}")]
pub enum BroadcastChatMessageError
{
	#[display("monitor is unavailable")]
	MonitorUnavailable,

	#[display("server is not connected")]
	ServerNotConnected,
}

/// Messages sent from [`ServerMonitorHandle`] to [`ServerMonitor`]
#[derive(Debug)]
enum MonitorMessage
{
	/// Informs the monitor of a server trying to establish a connection.
	ServerConnecting
	{
		/// The ID of the server that is trying to connect
		id: ServerId,

		/// The HTTP upgrade sent by the client
		upgrade: WebSocketUpgrade,

		/// Channel for the monitor to use to send a response back
		response_tx: oneshot::Sender<Result<Response, ServerConnectingError>>,
	},

	/// Tells the monitor to close the connection to a specific server, if there
	/// is any.
	DisconnectServer
	{
		/// The ID of the server that should be disconnected
		id: ServerId,
	},

	/// Retrieves realtime connection information from one or more servers.
	WantConnectionInfo
	{
		/// The ID of the server to get connection info from or [`None`] if info
		/// from all servers is requested
		server_id: Option<ServerId>,

		/// Channel for the server task(s) to use to send a response back
		response_tx: mpsc::UnboundedSender<Option<servers::ConnectionInfo>>,
	},

	/// Instructs servers to broadcast a chat message.
	BroadcastChatMessage
	{
		/// The ID of the server to broadcast the message to or [`None`] if it
		/// should be broadcasted to all servers
		server_id: Option<ServerId>,

		/// The message to broadcast
		message: Box<str>,

		/// Channel for the server task(s) to use to send a response back
		///
		/// `true` means the message has been sent to the server.
		response_tx: mpsc::UnboundedSender<bool>,
	},
}

/// Messages sent from [`ServerMonitor`] to server tasks
#[derive(Debug, Clone)]
enum ServerMessage
{
	/// Instructs a connection to terminate immediately.
	Disconnect,

	/// Retrieves realtime connection information from one or more servers.
	WantConnectionInfo
	{
		/// Channel for the server(s) to use to send a response back
		response_tx: mpsc::UnboundedSender<Option<servers::ConnectionInfo>>,
	},

	/// Instructs a server to broadcast a chat message.
	BroadcastChatMessage
	{
		/// The message to broadcast
		message: Arc<str>,

		/// Channel for the server task to use to send a response back
		///
		/// `true` means the message has been sent to the server.
		response_tx: mpsc::UnboundedSender<bool>,
	},
}

impl ServerMonitor
{
	pub fn new(database: Database, config: ServerMonitorConfig) -> Self
	{
		let (tx, rx) = mpsc::channel(128);
		let connected_servers = ConnectedServers::default();

		Self { tx, rx, database, config, connected_servers }
	}

	pub fn handle(&self) -> ServerMonitorHandle
	{
		ServerMonitorHandle { tx: self.tx.downgrade() }
	}

	#[tracing::instrument(skip(self, cancellation_token), err)]
	pub async fn run(
		mut self,
		cancellation_token: CancellationToken,
	) -> Result<(), ServerMonitorError>
	{
		loop {
			select! {
				() = cancellation_token.cancelled() => {
					tracing::info!("server monitor shutting down");
					return Ok(());
				},

				Some(message) = self.rx.recv() => match message {
					MonitorMessage::ServerConnecting { id, upgrade, response_tx } => {
						self.on_server_connecting(
							cancellation_token.child_token(),
							id,
							upgrade,
							response_tx,
						);
					},
					MonitorMessage::DisconnectServer { id } => {
						self.disconnect_server(id).await;
					},
					MonitorMessage::WantConnectionInfo { server_id, response_tx } => {
						self.on_want_connection_info(server_id, response_tx).await;
					},
					MonitorMessage::BroadcastChatMessage { server_id, message, response_tx } => {
						self.on_broadcast_chat_message(server_id, message, response_tx).await;
					},
				},

				Some(task_data) = self.connected_servers.join_next() => {
					self.on_server_disconnect(task_data).await?;
				},
			}
		}
	}

	#[tracing::instrument(level = "debug", skip(self, http_upgrade, response_tx))]
	fn on_server_connecting(
		&mut self,
		cancellation_token: CancellationToken,
		server_id: ServerId,
		http_upgrade: WebSocketUpgrade,
		response_tx: oneshot::Sender<Result<Response, ServerConnectingError>>,
	)
	{
		let connect_result = self
			.connected_servers
			.insert(server_id, http_upgrade, self.config, self.database.clone(), cancellation_token)
			.map_err(|task_id| {
				tracing::warn!(%server_id, %task_id, "server attempted to connect multiple times");
				ServerConnectingError::ServerAlreadyConnected
			});

		let _ = response_tx.send(connect_result);
	}

	#[tracing::instrument(level = "debug", skip(self))]
	async fn disconnect_server(&mut self, id: ServerId)
	{
		match self.connected_servers.send_message(id, ServerMessage::Disconnect).await {
			Ok(()) => {
				tracing::debug!(%id, "disconnected server");
			},
			Err(SendServerMessageError::ServerNotConnected { .. }) => {
				tracing::debug!(%id, "server was not connected");
			},
		}
	}

	#[tracing::instrument(level = "debug", skip(self, response_tx))]
	async fn on_want_connection_info(
		&mut self,
		server_id: Option<ServerId>,
		response_tx: mpsc::UnboundedSender<Option<servers::ConnectionInfo>>,
	)
	{
		let message = ServerMessage::WantConnectionInfo { response_tx };
		let Some(server_id) = server_id else {
			let server_count = self.connected_servers.broadcast_message(&message).await;
			tracing::debug!(server_count, "broadcasted message to all servers");
			return;
		};

		if let Err(SendServerMessageError::ServerNotConnected { message }) =
			self.connected_servers.send_message(server_id, message).await
		{
			let ServerMessage::WantConnectionInfo { response_tx } = message else {
				unreachable!();
			};

			let _ = response_tx.send(None);
		}
	}

	#[tracing::instrument(level = "debug", skip(self, response_tx))]
	async fn on_broadcast_chat_message(
		&mut self,
		server_id: Option<ServerId>,
		message: Box<str>,
		response_tx: mpsc::UnboundedSender<bool>,
	)
	{
		let message = ServerMessage::BroadcastChatMessage { message: message.into(), response_tx };
		let Some(server_id) = server_id else {
			let server_count = self.connected_servers.broadcast_message(&message).await;
			tracing::debug!(server_count, "broadcasted message to all servers");
			return;
		};

		if let Err(SendServerMessageError::ServerNotConnected { message }) =
			self.connected_servers.send_message(server_id, message).await
		{
			let ServerMessage::BroadcastChatMessage { response_tx, .. } = message else {
				unreachable!();
			};

			let _ = response_tx.send(false);
		}
	}

	#[tracing::instrument(level = "debug", skip(self), err)]
	async fn on_server_disconnect(
		&mut self,
		task_data: ServerTaskData,
	) -> Result<(), ServerMonitorError>
	{
		if !task_data.abort_handle.is_finished() {
			tracing::warn!("server disconnected but task is not finished?");
		}

		if let sender_count @ 1.. = task_data.tx.strong_count() {
			tracing::warn!(sender_count, "server disconnected but channel is not closed?");
		}

		event_queue::dispatch(Event::ServerDisconnected { id: task_data.id });

		Ok(())
	}
}

impl ServerMonitorHandle
{
	/// Creates a dangling handle.
	///
	/// Calls to this handle will always return a "monitor unavailable" error.
	pub fn dangling() -> Self
	{
		let (tx, _) = mpsc::channel(1);
		Self { tx: tx.downgrade() }
	}

	/// Notifies the monitor of a connecting server.
	#[tracing::instrument(skip(self, http_upgrade), err(level = "debug"))]
	pub async fn server_connecting(
		&self,
		server_id: ServerId,
		http_upgrade: WebSocketUpgrade,
	) -> Result<Response, ServerConnectingError>
	{
		let (response_tx, response_rx) = oneshot::channel();
		let monitor_tx = self.tx.upgrade().ok_or(ServerConnectingError::MonitorUnavailable)?;

		monitor_tx
			.send(MonitorMessage::ServerConnecting {
				id: server_id,
				upgrade: http_upgrade,
				response_tx,
			})
			.await
			.map_err(|_| ServerConnectingError::MonitorUnavailable)?;

		drop(monitor_tx);

		match response_rx.await {
			Ok(result) => result,
			Err(_) => Err(ServerConnectingError::MonitorUnavailable),
		}
	}

	/// Tells the monitor to close the connection to a specific server, if there
	/// is any.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn disconnect_server(&self, id: ServerId) -> Result<(), DisconnectServerError>
	{
		let monitor_tx = self.tx.upgrade().ok_or(DisconnectServerError::MonitorUnavailable)?;

		monitor_tx
			.send(MonitorMessage::DisconnectServer { id })
			.await
			.map_err(|_| DisconnectServerError::MonitorUnavailable)
	}

	/// Retrieves realtime connection information from a server.
	#[tracing::instrument(skip(self), ret(level = "debug"), err(level = "debug"))]
	pub async fn get_connection_info(
		&self,
		server_id: ServerId,
	) -> Result<Option<servers::ConnectionInfo>, GetConnectionInfoError>
	{
		let (response_tx, mut response_rx) = mpsc::unbounded_channel();
		let monitor_tx = self.tx.upgrade().ok_or(GetConnectionInfoError::MonitorUnavailable)?;

		monitor_tx
			.send(MonitorMessage::WantConnectionInfo { server_id: Some(server_id), response_tx })
			.await
			.map_err(|_| GetConnectionInfoError::MonitorUnavailable)?;

		drop(monitor_tx);

		response_rx.recv().await.ok_or(GetConnectionInfoError::MonitorUnavailable)
	}

	/// Instructs one or more servers to broadcast a chat message.
	///
	/// Returns how many servers have been instructed successfully.
	#[tracing::instrument(skip(self, message), err(level = "debug"))]
	pub async fn broadcast_chat_message(
		&self,
		message: Box<str>,
		server_id: Option<ServerId>,
	) -> Result<usize, BroadcastChatMessageError>
	{
		let (response_tx, response_rx) = mpsc::unbounded_channel();
		let monitor_tx = self.tx.upgrade().ok_or(BroadcastChatMessageError::MonitorUnavailable)?;

		tracing::debug!(chat_message = &*message, "broadcasting chat message");

		monitor_tx
			.send(MonitorMessage::BroadcastChatMessage { server_id, message, response_tx })
			.await
			.map_err(|_| BroadcastChatMessageError::MonitorUnavailable)?;

		drop(monitor_tx);

		let responses = stream::unfold(response_rx, async |mut response_rx| {
			response_rx.recv().await.map(|received| (received, response_rx))
		});

		Ok(responses.filter(|&received| future::ready(received)).count().await)
	}
}
