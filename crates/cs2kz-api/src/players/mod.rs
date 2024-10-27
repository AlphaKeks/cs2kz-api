//! KZ players.

use std::net::Ipv4Addr;

use futures_util::TryStreamExt;

use crate::database::{self, RowStream};
use crate::time::Timestamp;

mod player_id;
pub use player_id::PlayerID;

/// A KZ player.
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct Player {
	/// The player's SteamID.
	pub id: PlayerID,

	/// The player's Steam name.
	pub name: Box<str>,

	/// The player's IP address.
	///
	/// Only include it if you have permission to view it.
	#[serde(skip_serializing_if = "Option::is_none")]
	#[schema(value_type = Option<String>)]
	pub ip_address: Option<Ipv4Addr>,

	/// When the player joined their first CS2KZ server.
	pub first_joined_at: Timestamp,
}

/// A player's in-game preferences.
///
/// These are arbitrary key-value pairs. The [cs2kz-metamod] plugin controls
/// them, we just store them.
///
/// [cs2kz-metamod]: https://github.com/KZGlobalTeam/cs2kz-metamod
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[serde(transparent)]
pub struct Preferences(json::Map<String, json::Value>);

/// Returns at most `limit` players with the specified offset.
///
/// The first element of the returned tuple indicates how many players are in
/// the database. The second element is a stream that will yield no more than
/// `limit` values.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn get_players(
	conn: &mut database::Connection,
	limit: u64,
	offset: i64,
) -> database::Result<(u64, impl RowStream<'_, Player>)> {
	let total: u64 = sqlx::query_scalar!("SELECT COUNT(*) FROM Players")
		.fetch_one(conn.as_mut())
		.await?
		.try_into()
		.expect("`COUNT()` should return a non-negative value");

	let stream = sqlx::query_as!(
		Player,
		"SELECT
		   id `id: PlayerID`,
		   name,
		   ip_address `ip_address: Option<Ipv4Addr>`,
		   created_at `first_joined_at: Timestamp`
		 FROM Players
		 ORDER BY created_at DESC
		 LIMIT ?
		 OFFSET ?",
		limit,
		offset,
	)
	.fetch(conn.as_mut())
	.map_err(Into::into);

	Ok((total, stream))
}

/// Returns the player with the given ID.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn get_player_by_id(
	conn: &mut database::Connection,
	player_id: PlayerID,
) -> database::Result<Option<Player>> {
	sqlx::query_as!(
		Player,
		"SELECT
		   id `id: PlayerID`,
		   name,
		   ip_address `ip_address: Option<Ipv4Addr>`,
		   created_at `first_joined_at: Timestamp`
		 FROM Players
		 WHERE id = ?",
		player_id,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Returns the player with the given name.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn get_player_by_name(
	conn: &mut database::Connection,
	player_name: &str,
) -> database::Result<Option<Player>> {
	sqlx::query_as!(
		Player,
		"SELECT
		   id `id: PlayerID`,
		   name,
		   ip_address `ip_address: Option<Ipv4Addr>`,
		   created_at `first_joined_at: Timestamp`
		 FROM Players
		 WHERE name LIKE ?",
		format!("%{player_name}%"),
	)
	.fetch_optional(conn.as_mut())
	.await
	.map_err(Into::into)
}

/// Returns a player's in-game preferences.
#[instrument(level = "debug", skip(conn), err(level = "debug"))]
pub async fn get_preferences(
	conn: &mut database::Connection,
	player_id: PlayerID,
) -> database::Result<Option<Preferences>> {
	sqlx::query_scalar!(
		"SELECT preferences `preferences: sqlx::types::Json<Preferences>`
		 FROM Players
		 WHERE id = ?",
		player_id,
	)
	.fetch_optional(conn.as_mut())
	.await
	.map(|maybe_row| maybe_row.map(|row| row.0))
	.map_err(Into::into)
}
