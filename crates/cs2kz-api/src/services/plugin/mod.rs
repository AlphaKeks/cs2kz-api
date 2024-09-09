//! A service to manager cs2kz-metamod versions.

use derive_more::{Constructor, Debug};

use crate::database;

mod models;
pub use models::{PluginVersionID, PluginVersionIdentifier, PluginVersionName};

pub mod submit_version;
pub mod get_version;
pub mod get_versions;
pub mod http;

/// A service to manage cs2kz-metamod versions.
#[derive(Debug, Clone, Constructor)]
pub struct PluginService
{
	#[debug("MySql")]
	mysql: database::Pool,
}
