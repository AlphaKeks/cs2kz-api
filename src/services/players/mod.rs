//! A service for managing KZ Players.

use axum::extract::FromRef;

mod error;
pub use error::{Error, Result};

mod models;
pub use models::PlayerInfo;

mod http;

/// A service for managing KZ Players.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct PlayerService {}

impl PlayerService
{
	/// Create a new [`PlayerService`].
	pub fn new() -> Self
	{
		Self {}
	}
}
