//! Module containing everything related to KZ admins.

use axum::Router;

use crate::State;

/// Returns a [Router] for `/admins`.
pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
