use std::io;

use axum::extract::ws;

use super::super::message::{DecodeMessageError, EncodeMessageError};

/// Errors that can occur when establishing a connection.
#[derive(Debug, Error)]
pub enum EstablishConnectionError
{
	/// We failed to encode a message.
	#[error(transparent)]
	EncodeMessage(#[from] EncodeMessageError),

	/// We failed to receive a message from the socket.
	#[error("{0}")]
	Recv(#[source] io::Error),

	/// We failed to send a message over the socket.
	#[error("{0}")]
	Send(#[source] io::Error),

	/// The client closed the connection.
	#[error("connection closed unexpectedly")]
	ConnectionClosed
	{
		/// The close frame sent by the client (if any).
		frame: Option<ws::CloseFrame<'static>>,
	},
}

/// Errors that can occur when serving a connection.
#[derive(Debug, Error)]
pub enum ServeConnectionError
{
	/// We failed to encode a message.
	#[error(transparent)]
	EncodeMessage(#[from] EncodeMessageError),

	/// We failed to send a message over the socket.
	#[error(transparent)]
	Send(#[from] SendMessageError),

	/// We failed to receive a message from the socket.
	#[error(transparent)]
	Recv(#[from] ReceiveMessageError),
}

/// Errors that can occur when sending a message through a [`Connection`].
///
/// [`Connection`]: super::Connection
#[derive(Debug, Error)]
pub enum SendMessageError
{
	/// The underlying I/O transport failed.
	#[error(transparent)]
	Io(#[from] io::Error),

	/// We failed to encode our message (this is a bug!).
	#[error(transparent)]
	EncodeMessage(#[from] EncodeMessageError),
}

/// Errors that can occur when receiving a message from a [`Connection`].
///
/// [`Connection`]: super::Connection
#[derive(Debug, Error)]
pub enum ReceiveMessageError
{
	/// The underlying I/O transport failed.
	#[error(transparent)]
	Io(#[from] io::Error),

	/// We failed to decode the client's message.
	#[error(transparent)]
	DecodeMessage(#[from] DecodeMessageError),
}
