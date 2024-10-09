use futures::stream::{Stream, TryStreamExt};

use super::Player;
use crate::database::{self, Limit, Offset};
use crate::players::{PlayerID, PlayerName};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	player_id: PlayerID,
) -> database::Result<Option<Player>> {
	let player = macros::query!("WHERE id = ? ORDER BY created_at DESC", player_id)
		.fetch_optional(conn)
		.await?
		.map(|row| macros::map_row!(row));

	Ok(player)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_name(
	conn: &mut database::Connection,
	player_name: &PlayerName,
) -> database::Result<Option<Player>> {
	let player_name = format!("%{player_name}%");
	let player = macros::query!("WHERE name LIKE ? ORDER BY created_at DESC", player_name)
		.fetch_optional(conn)
		.await?
		.map(|row| macros::map_row!(row));

	Ok(player)
}

#[instrument(skip(conn))]
pub fn get<'c>(
	conn: &'c mut database::Connection,
	GetPlayersParams {
		name,
		limit,
		offset,
	}: GetPlayersParams<'_>,
) -> impl Stream<Item = database::Result<Player>> + Unpin + Send + 'c {
	macros::query! {
		"WHERE name LIKE COALESCE(?, name)
		 ORDER BY created_at DESC
		 LIMIT ?
		 OFFSET ?",
		name.map(|name| format!("%{name}%")),
		limit,
		offset,
	}
	.fetch(conn)
	.map_ok(|row| macros::map_row!(row))
}

pub struct GetPlayersParams<'a> {
	pub name: Option<&'a PlayerName>,
	pub limit: Limit,
	pub offset: Offset,
}

mod macros {
	macro_rules! query {
		($extra_query:literal, $($args:tt)*) => {
			sqlx::query! {
				"SELECT
				   id `id: PlayerID`,
				   name `name: PlayerName`,
				   ip_address `ip_address: std::net::Ipv4Addr`,
				   preferences `preferences: sqlx::types::Json<json::Map<String, json::Value>>`,
				   created_at,
				   last_seen_at
				 FROM Players "
				+ $extra_query,
				$($args)*
			}
		};
	}

	macro_rules! map_row {
		($row:ident) => {
			$crate::players::database::Player {
				id: $row.id,
				name: $row.name,
				ip_address: $row.ip_address,
				preferences: $row.preferences.0,
				created_at: $row.created_at,
				last_seen_at: $row.last_seen_at,
			}
		};
	}

	pub(super) use {map_row, query};
}
