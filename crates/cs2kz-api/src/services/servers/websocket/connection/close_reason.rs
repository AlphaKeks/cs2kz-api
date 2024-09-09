use std::borrow::Cow;

use axum::extract::ws;

/// Reasons to close a WebSocket connection.
#[derive(Debug)]
pub enum CloseReason
{
	/// The client did something wrong.
	ClientError
	{
		/// An error message to send to the client.
		message: Cow<'static, str>,
	},

	/// The client has not sent a heartbeat within the required timeout.
	Timeout,

	/// The task managing the connection has been cancelled.
	Cancelled,
}

impl CloseReason
{
	/// Encodes this [`CloseReason`] as a [`ws::CloseFrame`] with an appropriate error code and
	/// message.
	pub fn as_close_frame(&self) -> ws::CloseFrame<'static>
	{
		// https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent/code#value
		let (code, reason) = match self {
			Self::ClientError { message } => (1008, message.clone()),
			Self::Timeout => (1008, Cow::Borrowed("connection timeout")),
			Self::Cancelled => (1012, Cow::Borrowed("API is shutting down")),
		};

		ws::CloseFrame { code, reason }
	}
}
