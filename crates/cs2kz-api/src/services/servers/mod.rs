//! This module contains the [`ServerService`].

use derive_more::{Constructor, Debug};

use crate::database;

mod models;
pub use models::{AccessKey, Host, ServerID, ServerIdentifier, ServerOwner};

pub mod register_server;
pub mod get_server;
pub mod get_servers;
pub mod update_server;
pub mod get_access_key;
pub mod reset_access_key;
pub mod clear_access_key;
pub mod websocket;
pub mod http;

/// A service to manage CS2 servers.
#[derive(Debug, Clone, Constructor)]
pub struct ServerService
{
	#[debug("MySql")]
	mysql: database::Pool,
}
