//! WebSocket connections to running CS2 servers.
//!
//! Every CS2 server with an API key will establish a WebSocket connection with
//! the API while it's running. This connection is then used for all
//! communication. The flow is as follows:
//!
//! 1. The client makes an HTTP GET request to `/servers/websocket`, with their
//!    API key in an `Authorization` header.
//! 2. If authentication is successful, the server will respond with status 101
//!    and the WebSocket connection is established.
//! 3. The server waits for the client to send a [`Hello`] message, to which it
//!    will respond with a [`HelloAck`] message.
//! 4. After this handshake is complete, the client can send [`Message`]s, to
//!    which the server may or may not respond. The server may also send
//!    messages on its own, to which the client should react accordingly.
//!
//! [`Hello`]: message::Incoming::Hello
//! [`HelloAck`]: message::Outgoing::HelloAck

use cs2kz::SteamID;
use serde::Deserialize;

use crate::net::IpAddr;

mod close_reason;
pub use close_reason::CloseReason;

mod message;

pub mod connection;
pub use connection::Connection;

/// Information about a connected player.
#[derive(Debug, Deserialize)]
pub struct PlayerInfo
{
	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's name.
	pub name: String,

	/// The player's IP address.
	pub ip_addr: IpAddr,
}
