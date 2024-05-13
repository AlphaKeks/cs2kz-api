//! Shared data types for this module.

use std::net::{Ipv4Addr, SocketAddrV4};

use chrono::{DateTime, Utc};
use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

use crate::authentication;
use crate::players::Player;

#[cs2kz_api_macros::id]
pub struct ServerID(pub u16);

/// A CS2 server.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Server {
	/// The server's ID.
	pub id: ServerID,

	/// The server's name.
	pub name: String,

	/// The server's IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,

	/// The server's port.
	pub port: u16,

	/// The server's owner.
	#[sqlx(flatten)]
	pub owner: Player,

	/// When this server was approved.
	pub created_on: DateTime<Utc>,
}

/// Request payload for approving a new CS2 server.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewServer {
	/// The server's name.
	pub name: String,

	/// The server's IP address.
	#[schema(value_type = String)]
	pub ip_address: SocketAddrV4,

	/// The server owner's SteamID.
	pub owned_by: SteamID,
}

/// Response body for approving a new CS2 server.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedServer {
	/// The server's ID.
	pub server_id: ServerID,

	/// The server's API key.
	pub key: authentication::server::Key,
}

/// Request payload for generating a new access token.
#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRequest {
	/// The server's API key.
	pub key: authentication::server::Key,

	/// The CS2KZ version the server is currently running.
	pub plugin_version: semver::Version,
}

/// Request payload for updating a CS2 server.
#[derive(Debug, Deserialize, ToSchema)]
pub struct ServerUpdate {
	/// A new name.
	#[serde(with = "crate::serde::empty_as_none::string")]
	pub name: Option<String>,

	/// A new IP address.
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<SocketAddrV4>,

	/// A new server owner.
	pub owned_by: Option<SteamID>,
}

impl ServerUpdate {
	/// Checks if this update only contains empty values.
	pub const fn is_empty(&self) -> bool {
		let Self {
			name,
			ip_address,
			owned_by,
		} = self;

		name.is_none() && ip_address.is_none() && owned_by.is_none()
	}
}

/// Response body for generating a new API key for a CS2 server.
#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
pub struct CreatedServerKey {
	/// The server's API key.
	pub key: authentication::server::Key,
}
