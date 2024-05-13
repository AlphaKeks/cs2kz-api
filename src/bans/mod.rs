//! Module containing everything related to player bans.

use axum::Router;

use crate::State;

/// Returns a [Router] for `/bans`.
pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
