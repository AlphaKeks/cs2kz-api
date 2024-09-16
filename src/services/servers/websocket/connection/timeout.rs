//! A simple timeout for WebSocket connections.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use tokio::time::{sleep, Duration, Instant, Sleep};

/// A simple timeout for WebSocket connections.
pub struct Timeout
{
	/// How long to wait.
	duration: Duration,

	/// The timeout future.
	sleep: Pin<Box<Sleep>>,
}

impl Timeout
{
	/// Creates a new [`Timeout`]
	pub fn new(duration: Duration) -> Self
	{
		Self { duration, sleep: Box::pin(sleep(duration)) }
	}

	/// Returns a [`Future`] which completes once the timeout has elapsed.
	pub fn wait(&mut self) -> impl Future<Output = ()> + '_
	{
		&mut self.sleep
	}

	/// Resets the timeout.
	pub fn reset(&mut self)
	{
		self.sleep.as_mut().reset(Instant::now() + self.duration);
	}
}

impl fmt::Debug for Timeout
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Timeout")
			.field("duration", &self.duration)
			.field("deadline", &self.sleep.deadline())
			.finish()
	}
}
