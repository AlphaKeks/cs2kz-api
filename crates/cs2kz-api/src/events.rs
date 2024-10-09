//! A global event queue.
//!
//! [`Event`]s can [`dispatch`]ed from anywhere in the codebase.
//! The primary consumer of those events is the `/events` HTTP endpoint, where we forward events to
//! WebSocket clients.

use std::panic::Location;
use std::sync::LazyLock;

use tokio::sync::broadcast;

use crate::git::GitRevision;
use crate::plugin_versions::{PluginVersionID, PluginVersionName};
use crate::users::{UserID, UserUpdate};

const QUEUE_SIZE: usize = 128;

static QUEUE: LazyLock<broadcast::Sender<Event>> = LazyLock::new(|| {
	let (tx, _) = broadcast::channel::<Event>(QUEUE_SIZE);
	tx
});

#[instrument(level = "trace")]
pub fn dispatch(event: Event) {
	if let Err(event) = QUEUE.send(event) {
		debug!("no event listeners");
		trace!(?event);
	}
}

#[track_caller]
#[instrument(level = "trace", fields(location = %Location::caller()))]
pub fn subscribe() -> broadcast::Receiver<Event> {
	QUEUE.subscribe()
}

/// Returns the amount of currently active subscribers.
pub fn subscriber_count() -> usize {
	QUEUE.receiver_count()
}

#[derive(Debug, Clone)]
pub enum Event {
	/// A new user has been registered.
	UserRegistered { user_id: UserID },

	/// Information about an existing user has been updated.
	UserUpdated(UserUpdate),

	/// A new cs2kz-metamod version has been released.
	PluginVersionSubmitted {
		id: PluginVersionID,
		name: PluginVersionName,
		git_revision: GitRevision,
	},
}
