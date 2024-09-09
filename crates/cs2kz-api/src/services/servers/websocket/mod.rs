//! CS2 server WebSocket connections.
//!
//! Every CS2 server with an API key will establish a WebSocket connection with the API while it's
//! running. This connection is then used for all communication. The flow is as follows:
//!
//! 1. The client makes an HTTP GET request to `/servers/websocket`, with their API key in an
//!    `Authorization` header.
//! 2. If authentication is successful, the server will respond with status 101 and the WebSocket
//!    connection is established.
//! 3. The server waits for the client to send a [`Hello`] message, to which it will respond with
//!    a [`HelloAck`] message.
//! 4. After this handshake is complete, the client can send [`Message`]s, to which the server may
//!    or may not respond. The server may also send messages on its own, to which the client should
//!    react accordingly.
//!
//! [`Hello`]: handshake::Hello
//! [`HelloAck`]: handshake::HelloAck

use std::io;
use std::time::Duration;

use axum::extract::ws;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::select;
use tokio_util::sync::CancellationToken;

use self::connection::{CloseReason, ReceiveMessageError, ServeConnectionError, Timeout};
use self::message::DecodeMessageError;

mod handshake;

mod message;
pub use message::Message;

pub mod connection;
pub use connection::Connection;

/// Serves the given [`Connection`].
///
/// This function will not return unless cancelled via the provided [`CancellationToken`], or if
/// there is an error.
pub async fn serve_connection<T>(
	mut conn: Connection<T>,
	heartbeat_interval: Duration,
	cancel: CancellationToken,
) -> Result<(), ServeConnectionError>
where
	T: Stream<Item = io::Result<ws::Message>>,
	T: Sink<ws::Message, Error = io::Error>,
	T: Send + Unpin,
{
	let mut timeout = Timeout::new(heartbeat_interval);

	loop {
		select! {
			biased;

			// Server is shutting down.
			() = cancel.cancelled() => conn.close(CloseReason::Cancelled).await?,

			// The client didn't send heartbeats.
			() = timeout.wait() => conn.close(CloseReason::Timeout).await?,

			Some(message) = conn.next() => match message {
				Ok(message) => {
					handle_message(&mut conn, &mut timeout, message).await?;
				},
				Err(error @ ReceiveMessageError::Io(_)) => {
					return Err(ServeConnectionError::Recv(error));
				},
				Err(ReceiveMessageError::DecodeMessage(DecodeMessageError::InvalidJSON {
					id, source
				})) => {
					conn.send(Message::error(&source, id)).await?;
				},
				Err(ReceiveMessageError::DecodeMessage(error)) => {
					conn.send(Message::error(&error, None)).await?;
				},
			},

			// Socket sent [`None`] -> we're done
			else => break Ok(()),
		}
	}
}

#[allow(unused, dead_code)]
async fn handle_message<T>(
	conn: &mut Connection<T>,
	timeout: &mut Timeout,
	message: Message<message::Incoming>,
) -> Result<(), ServeConnectionError>
where
	T: Stream<Item = io::Result<ws::Message>>,
	T: Sink<ws::Message, Error = io::Error>,
	T: Send + Unpin,
{
	use message::Incoming as M;

	let message_id = message.id();

	match message.payload() {
		M::Heartbeat { players } => {
			timeout.reset();
			info!(?players, "received heartbeat");
		}
	}

	Ok(())
}
