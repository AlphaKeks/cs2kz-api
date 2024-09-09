//! This module contains the [`Connection`] type, a wrapper around arbitrary WebSocket-like I/O
//! resources. It can send and receive [`Message`]s rather than raw [`ws::Message`]s, and its
//! constructor performs our initial handshake.

use std::io;
use std::pin::Pin;
use std::task::{ready, Poll};

use axum::extract::ws;
use futures::{Sink, SinkExt, Stream, TryStreamExt};

use super::handshake::{Hello, HelloAck};
use super::message::{self, DecodeMessageError, Message};

mod errors;
pub use errors::{
	EstablishConnectionError,
	ReceiveMessageError,
	SendMessageError,
	ServeConnectionError,
};

mod timeout;
pub use timeout::Timeout;

mod close_reason;
pub use close_reason::CloseReason;

/// A live WebSocket connection.
#[pin_project]
#[derive(Debug)]
pub struct Connection<T>
{
	/// The underlying socket.
	///
	/// This is generic so we can mock it in tests.
	#[pin]
	socket: T,
}

impl<T> Connection<T>
where
	T: Stream<Item = io::Result<ws::Message>>,
	T: Sink<ws::Message, Error = io::Error>,
	T: Send + Unpin,
{
	/// Establishes the connection by performing the initial handshake.
	pub async fn establish(mut socket: T) -> Result<Self, EstablishConnectionError>
	{
		let hello = loop {
			let raw = socket
				.try_next()
				.await
				.map_err(EstablishConnectionError::Recv)?
				.ok_or(EstablishConnectionError::ConnectionClosed { frame: None })?;

			match Hello::decode(&raw) {
				Ok(hello) => {
					break hello;
				}
				Err(DecodeMessageError::NotJSON | DecodeMessageError::InvalidJSON { .. }) => {
					continue;
				}
				Err(DecodeMessageError::ConnectionClosed { frame }) => {
					return Err(EstablishConnectionError::ConnectionClosed { frame });
				}
			}
		};

		let hello_ack = HelloAck::new(&hello).encode()?;

		if let Err(error) = socket.send(hello_ack).await {
			error!(?error, "failed to ACK hello");
			return Err(EstablishConnectionError::Send(error));
		}

		Ok(Self { socket })
	}
}

impl<T> Connection<T>
where
	T: Sink<ws::Message, Error = io::Error>,
	T: Send + Unpin,
{
	/// Closes the connection.
	pub async fn close(&mut self, reason: CloseReason) -> Result<(), ServeConnectionError>
	{
		self.socket
			.send(ws::Message::Close(Some(reason.as_close_frame())))
			.await
			.map_err(SendMessageError::Io)
			.map_err(Into::into)
	}
}

impl<T> Stream for Connection<T>
where
	T: Stream<Item = io::Result<ws::Message>>,
	T: Send + Unpin,
{
	type Item = Result<Message<message::Incoming>, ReceiveMessageError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>)
		-> Poll<Option<Self::Item>>
	{
		let Some(raw) = ready!(self.project().socket.poll_next(cx)?) else {
			return Poll::Ready(None);
		};

		Poll::Ready(Some(Message::decode(&raw).map_err(Into::into)))
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		self.socket.size_hint()
	}
}

impl<T> Sink<Message<message::Outgoing>> for Connection<T>
where
	T: Sink<ws::Message, Error = io::Error>,
	T: Send + Unpin,
{
	type Error = SendMessageError;

	fn poll_ready(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		self.project().socket.poll_ready(cx).map_err(Into::into)
	}

	fn start_send(self: Pin<&mut Self>, item: Message<message::Outgoing>)
		-> Result<(), Self::Error>
	{
		let raw = item.encode()?;

		self.project().socket.start_send(raw).map_err(Into::into)
	}

	fn poll_flush(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		self.project().socket.poll_flush(cx).map_err(Into::into)
	}

	fn poll_close(
		self: Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> Poll<Result<(), Self::Error>>
	{
		self.project().socket.poll_close(cx).map_err(Into::into)
	}
}
