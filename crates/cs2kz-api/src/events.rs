//! The API's internal pub/sub event system.
//!
//! Whenever something "significant" happens that certain tasks may be interested in, an [`Event`]
//! is [`dispatch`]ed. Tasks that have previously called [`subscribe()`] can now listen for those
//! events.
//!
//! This is (currently) mainly used for a public WebSocket endpoint that clients (such as websites)
//! can connect to to receive these events.

use std::panic::Location;
use std::sync::LazyLock;

use tokio::sync::broadcast;

const QUEUE_SIZE: usize = 16;

#[expect(unused)]
static QUEUE: LazyLock<broadcast::Sender<Event>> = LazyLock::new(|| {
	info!(capacity = QUEUE_SIZE, "initializing event queue");
	broadcast::channel(QUEUE_SIZE).0
});

/// An API event.
///
/// See the [module-level documentation] for detailed information.
///
/// [module-level documentation]: crate::events
#[derive(Debug, Clone)]
#[allow(
	dead_code,
	reason = "most of the data here will never be used directly, but included in tracing \
	          and/or exposed to consumers of the API"
)]
pub enum Event {}

/// Dispatches an event for subscribers to consume.
#[instrument(level = "trace")]
pub fn dispatch(event: Event) {
	if let Err(event) = QUEUE.send(event) {
		debug!(?event, "no event listeners");
	}
}

/// Subscribes to the event queue.
#[track_caller]
#[instrument(level = "trace", fields(location = %Location::caller()))]
pub fn subscribe() -> broadcast::Receiver<Event> {
	QUEUE.subscribe()
}
