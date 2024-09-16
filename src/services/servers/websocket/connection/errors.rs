//! Errors used in the [`connection`] module.
//!
//! [`connection`]: super

use axum::extract::ws;
use thiserror::Error;

use super::super::message::{DecodeMessageError, EncodeMessageError};
use crate::services;

/// Errors that can occur while receiving messages.
#[derive(Debug, Error)]
pub enum ReceiveMessageError
{
	/// The client closed the connection.
	#[error("client closed connection unexpectedly")]
	ConnectionClosed
	{
		/// The close frame that was included in the message, if any.
		close_frame: Option<ws::CloseFrame<'static>>,
	},

	/// We failed to decode the client's message.
	#[error("failed to decode message: {0}")]
	DecodeMessage(DecodeMessageError),

	/// The underlying stream returned an error.
	#[error("failed to receive message from stream: {0}")]
	Io(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl From<DecodeMessageError> for ReceiveMessageError
{
	fn from(error: DecodeMessageError) -> Self
	{
		match error {
			error @ (DecodeMessageError::ParseJson(_) | DecodeMessageError::NotJson) => {
				Self::DecodeMessage(error)
			}
			DecodeMessageError::ConnectionClosed { close_frame } => {
				Self::ConnectionClosed { close_frame }
			}
		}
	}
}

/// Errors that can occur while sending messages.
#[derive(Debug, Error)]
pub enum SendMessageError
{
	/// We failed to encode a message.
	#[error("failed to encode message; this is a bug!")]
	EncodeMessage(#[from] EncodeMessageError),

	/// The underlying stream returned an error.
	#[error("failed to send message into stream: {0}")]
	Io(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// Errors that can occur during the initial handshake.
#[derive(Debug, Error)]
pub enum HandshakeError
{
	/// The client didn't shake hands in time.
	#[error("handshake did not complete within the timeout")]
	Timeout,

	/// The client closed the connection.
	#[error("client closed connection unexpectedly")]
	ConnectionClosed
	{
		/// The close frame that was included in the message, if any.
		close_frame: Option<ws::CloseFrame<'static>>,
	},

	/// We failed to encode the ACK.
	#[error("failed to encode message; this is a bug!")]
	EncodeAck(#[from] EncodeMessageError),

	/// The underlying stream returned an error.
	#[error("failed to send message into stream: {0}")]
	Io(#[source] Box<dyn std::error::Error + Send + Sync + 'static>),
}

/// Errors that can occur when establishing a new [`Connection`].
///
/// [`Connection`]: super::Connection
#[derive(Debug, Error)]
pub enum EstablishConnectionError
{
	/// The initial handshake failed.
	#[error(transparent)]
	Handshake(#[from] HandshakeError),
}

/// Errors that can occur when serving a [`Connection`].
///
/// [`Connection`]: super::Connection
#[derive(Debug, Error)]
pub enum ServeConnectionError
{
	/// We failed to send a message.
	#[error("failed to send message: {0}")]
	SendMessage(#[from] SendMessageError),

	/// The player service returned an error.
	#[error(transparent)]
	PlayerService(#[from] services::players::Error),
}
