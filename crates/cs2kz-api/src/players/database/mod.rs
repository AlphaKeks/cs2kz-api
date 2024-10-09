//! Functions to interact with the `Players` table.

use std::net::Ipv4Addr;

use time::OffsetDateTime;

use crate::players::{PlayerID, PlayerName};

mod create;
pub use create::{create, CreatePlayerError, NewPlayer};

mod get;
pub use get::{get, get_by_id, get_by_name};

pub struct Player {
	pub id: PlayerID,
	pub name: PlayerName,
	pub ip_address: Ipv4Addr,
	pub preferences: json::Map<String, json::Value>,
	pub created_at: OffsetDateTime,
	pub last_seen_at: OffsetDateTime,
}
