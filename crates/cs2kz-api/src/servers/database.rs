//! Functions to interact with the `Servers` table.

mod create;
pub use create::{create, CreateServerError, NewServer};

mod get;
pub use get::{get, get_by_access_key, get_by_id, get_by_name, Server};

mod update;
pub use update::{mark_seen, update, ServerUpdate, UpdateServerError};

mod access_key;
pub use access_key::{invalidate_access_key, reset_access_key};
