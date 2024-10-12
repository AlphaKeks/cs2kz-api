use std::num::NonZero;

mod name;
pub use name::ServerName;

mod host;
pub use host::ServerHost;

mod access_key;
pub use access_key::AccessKey;

mod database;
pub use database::{
	get,
	get_by_access_key,
	get_by_id,
	get_by_name,
	invalidate_access_key,
	mark_seen,
	reset_access_key,
	Server,
};

mod approve;
pub use approve::{approve, ApproveServerError, ApprovedServer, NewServer};

mod update;
pub use update::{update, ServerUpdate, UpdateServerError};

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
pub struct ServerID(NonZero<u16>);
