use axum::extract::FromRef;

use crate::database;

#[derive(Clone, FromRef)]
pub struct AppState {
	pub database: database::ConnectionPool,
}
