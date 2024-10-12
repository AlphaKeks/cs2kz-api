//! Everything related to [cs2kz-metamod] releases.
//!
//! Every official release is tracked by the API. CS2 servers will send their current version when
//! authenticating, so the API can perform checks and reject outdated servers. New versions are
//! submitted automatically by GitHub Actions.
//!
//! [cs2kz-metamod]: https://github.com/KZGlobalTeam/cs2kz-metamod

use std::num::NonZero;

use time::OffsetDateTime;

use crate::git::GitRevision;

mod version_name;
pub use version_name::PluginVersionName;

mod database;
pub use database::{
	get,
	get_by_git_revision,
	get_by_id,
	get_by_name,
	GetPluginVersionsParams,
	PluginVersion,
};

mod submit;
pub use submit::{submit, NewPluginVersion, SubmitPluginVersionError};

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	serde::Serialize,
	serde::Deserialize,
	sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PluginVersionID(NonZero<u16>);
