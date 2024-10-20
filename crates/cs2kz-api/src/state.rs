use axum::extract::FromRef;

use crate::database;

/// The global application state.
///
/// This is what we pass to [`axum::Router::with_state()`] at the top level.
/// Do note however that this type is never extracted directly. We `#[derive(FromRef)]` so that any
/// of the field types can be extracted using [`axum::extract::State`]. See the "Substates" section
/// of the [`axum::extract::State`] documentation for more details.
#[derive(Clone, FromRef)]
pub(crate) struct AppState {
	pub(crate) pool: database::ConnectionPool,
}
