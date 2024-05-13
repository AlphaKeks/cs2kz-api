//! Module containing everything related to jumpstats.

use axum::Router;

use crate::State;

/// Returns a [Router] for `/jumpstats`.
pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
