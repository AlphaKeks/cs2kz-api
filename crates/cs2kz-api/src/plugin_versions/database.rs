//! Functions to interact with the `PluginVersions` table.

use time::OffsetDateTime;

use crate::git::GitRevision;
use crate::plugin_versions::{PluginVersionID, PluginVersionName};

mod create;
pub use create::{create, CreatePluginVersionError};

mod get;
pub use get::{
	get,
	get_by_git_revision,
	get_by_id,
	get_by_name,
	get_latest,
	GetPluginVersionsParams,
};

#[derive(Debug, Clone, PartialEq)]
pub struct PluginVersion {
	pub id: PluginVersionID,
	pub name: PluginVersionName,
	pub git_revision: GitRevision,
	pub created_at: OffsetDateTime,
}
