//! A service for managing the CS2KZ plugin.

use axum::extract::FromRef;

use crate::util;

mod error;
pub use error::{Error, Result};

mod version;
pub use version::PluginVersion;

mod service;
mod http;

/// A service for managing KZ maps.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct PluginService {}

impl PluginService
{
	/// Create a new [`PluginService`].
	pub fn new() -> Self
	{
		Self {}
	}
}

util::make_id! {
	/// A unique identifier for CS2KZ versions.
	PluginVersionID as u16
}
