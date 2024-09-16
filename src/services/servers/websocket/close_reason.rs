//! Reasons why we want to close a WebSocket connection.

use std::borrow::Cow;

use axum::extract::ws;

/// Reasons why we want to close a WebSocket connection.
#[derive(Debug)]
pub enum CloseReason
{
	/// The server is shutting down, so we have to close all connections.
	ServerShutdown,

	/// The client did not send enough heartbeats.
	ClientTimeout,

	/// The server encountered an irrecoverable error.
	Error(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl CloseReason
{
	/// Encodes this [`CloseFrame`] as a [`ws::CloseFrame`].
	pub(super) fn as_close_frame(&self) -> ws::CloseFrame<'static>
	{
		// see: https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent/code#value
		let (code, reason) = match self {
			Self::ServerShutdown => (1012, Cow::Borrowed("API is shutting down")),
			Self::ClientTimeout => (1000, Cow::Borrowed("did not receive heartbeat in time")),
			Self::Error(error) => (1002, Cow::Owned(error.to_string())),
		};

		ws::CloseFrame { code, reason }
	}
}
