//! A service for managing records.

use axum::extract::FromRef;

mod error;
pub use error::{Error, Result};

mod models;

mod http;

/// A service for managing records.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct RecordService {}

impl RecordService
{
	/// Create a new [`RecordService`].
	pub fn new() -> Self
	{
		Self {}
	}
}
