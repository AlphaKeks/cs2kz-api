//! Shared data types for this module.

use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use cs2kz::SteamID;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlRow;
use sqlx::{FromRow, Row};
use utoipa::ToSchema;

use crate::maps::CourseID;
use crate::time::Seconds;

/// A KZ player.
#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Player {
	/// The player's name.
	#[sqlx(rename = "player_name")]
	pub name: String,

	/// The player's SteamID.
	#[sqlx(rename = "player_id")]
	pub steam_id: SteamID,
}

/// A KZ player (with more information).
#[derive(Debug, Serialize, ToSchema)]
pub struct FullPlayer {
	/// The base player object.
	#[serde(flatten)]
	pub player: Player,

	/// The player's IP address.
	pub ip_address: Option<Ipv4Addr>,

	/// Whether the player is currently banned.
	pub is_banned: bool,
}

impl FromRow<'_, MySqlRow> for FullPlayer {
	fn from_row(row: &MySqlRow) -> sqlx::Result<Self> {
		Ok(Self {
			player: Player::from_row(row)?,
			ip_address: row
				.try_get::<Option<&str>, _>("ip_address")?
				.map(|ip| {
					ip.parse::<Ipv4Addr>()
						.map_err(|err| sqlx::Error::ColumnDecode {
							index: String::from("ip_address"),
							source: Box::new(err),
						})
				})
				.transpose()?,
			is_banned: row.try_get("is_banned")?,
		})
	}
}

/// Request payload for creating new players.
#[derive(Debug, Deserialize, ToSchema)]
pub struct NewPlayer {
	/// The player's name.
	pub name: String,

	/// The player's SteamID.
	pub steam_id: SteamID,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,
}

/// Request payload for updating players.
#[derive(Debug, Deserialize, ToSchema)]
pub struct PlayerUpdate {
	/// The player's name.
	pub name: String,

	/// The player's IP address.
	#[schema(value_type = String)]
	pub ip_address: Ipv4Addr,

	/// The player's in-game preferences.
	#[serde(default)]
	pub preferences: JsonValue,

	/// Data about the player's session on the server.
	pub session_data: GameSessionData,
}

#[cs2kz_api_macros::id]
pub struct GameSessionID(pub u64);

/// Information about a player's session on a CS2 server.
///
/// A session begins when a player joins and ends when the player disconnects, or when the map
/// changes, whichever happens first.
#[derive(Debug, Deserialize, ToSchema)]
pub struct GameSessionData {
	/// The amount of time the player was actively playing.
	pub time_active: Seconds,

	/// The amount of time the player was spectating.
	pub time_spectating: Seconds,

	/// The amount of time the player was AFK.
	pub time_afk: Seconds,

	/// Information about individual course sessions.
	pub course_sessions: BTreeMap<CourseID, CourseSessionData>,
}

#[cs2kz_api_macros::id]
pub struct CourseSessionID(pub u64);

/// Information about a single course session.
///
/// Course sessions are collected in the same time frame as [game sessions], but with
/// course-specific data.
///
/// [game sessions]: GameSessionData
#[derive(Debug, Clone, Copy, Deserialize, ToSchema)]
pub struct CourseSessionData {
	/// The amount of time spent playing this course.
	pub playtime: Seconds,

	/// The amount of times the player has left the start zone.
	pub started_runs: u16,

	/// The amount of times the player has entered the end zone with a running timer.
	pub finished_runs: u16,
}

/// Response payload for [game sessions].
///
/// [game sessions]: GameSessionData
#[derive(Debug, Serialize, ToSchema)]
pub struct CreatedGameSession {
	/// The ID of the created game session.
	pub game_session_id: GameSessionID,

	/// The IDs of the created course sessions.
	pub course_session_ids: BTreeMap<CourseID, CourseSessionID>,
}
