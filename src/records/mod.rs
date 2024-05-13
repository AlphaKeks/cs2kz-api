//! Module containing everything related to records.

use axum::Router;

use crate::State;

/// Returns a [Router] for `/records`.
pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
