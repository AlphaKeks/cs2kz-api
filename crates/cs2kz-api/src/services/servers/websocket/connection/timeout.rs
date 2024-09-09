use std::future::Future;
use std::pin::Pin;

use derive_more::Debug;
use tokio::time::{Duration, Instant, Sleep};

/// A connection timeout.
#[derive(Debug)]
pub struct Timeout
{
	#[debug(skip)]
	sleep: Pin<Box<Sleep>>,
	duration: Duration,
}

impl Timeout
{
	/// Creates a new timeout of the given duration.
	pub fn new(duration: Duration) -> Self
	{
		Self {
			sleep: Box::pin(tokio::time::sleep(duration)),
			duration,
		}
	}

	/// Resets the timeout.
	pub fn reset(&mut self)
	{
		self.sleep.as_mut().reset(Instant::now() + self.duration);
	}

	/// Waits for the timeout to elapse.
	pub fn wait(&mut self) -> impl Future<Output = ()> + '_
	{
		&mut self.sleep
	}
}
