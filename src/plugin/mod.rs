//! Module containing everything related to the CS2KZ plugin.

use axum::Router;

use crate::State;

mod models;

#[doc(inline)]
pub use models::PluginVersionID;

/// Returns a [Router] for `/plugin`.
pub fn router(state: &'static State) -> Router {
	Router::new().with_state(state)
}
