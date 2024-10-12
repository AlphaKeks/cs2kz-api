use std::net::Ipv4Addr;

use futures::stream::{Stream, TryStreamExt};
use time::OffsetDateTime;

use crate::database::{self, RowStream};
use crate::pagination::{Limit, Offset, PaginationResults};
use crate::players::{PlayerID, PlayerName, Preferences};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get_by_id(
	conn: &mut database::Connection,
	player_id: PlayerID,
) -> database::Result<Option<Player>> {
	let player = sqlx::query!(
		"SELECT
		   id `id: PlayerID`,
		   name `name: PlayerName`,
		   ip_address `ip_address: std::net::Ipv4Addr`,
		   preferences `preferences: sqlx::types::Json<crate::players::Preferences>`,
		   created_at,
		   last_seen_at
		 FROM Players
		 WHERE id = ?
		 ORDER BY created_at DESC",
		player_id,
	)
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
	let player = sqlx::query!(
		"SELECT
		   id `id: PlayerID`,
		   name `name: PlayerName`,
		   ip_address `ip_address: std::net::Ipv4Addr`,
		   preferences `preferences: sqlx::types::Json<crate::players::Preferences>`,
		   created_at,
		   last_seen_at
		 FROM Players
		 WHERE name LIKE ?
		 ORDER BY created_at DESC",
		format!("%{player_name}%"),
	)
	.fetch_optional(conn)
	.await?
	.map(|row| macros::map_row!(row));

	Ok(player)
}

#[instrument(skip(conn), err(level = "debug"))]
pub async fn get<'c>(
	conn: &'c mut database::Connection,
	GetPlayersParams {
		name,
		limit,
		offset,
	}: GetPlayersParams<'_>,
) -> database::Result<PaginationResults<impl RowStream<'c, Player>>> {
	let name = name.map(|name| format!("%{name}%"));

	let total: u64 = sqlx::query_scalar!(
		"SELECT COUNT(id)
		 FROM Players
		 WHERE name LIKE COALESCE(?, name)",
		name,
	)
	.fetch_one(&mut *conn)
	.await?
	.try_into()
	.expect("`COUNT()` should return a positive value");

	let stream = sqlx::query!(
		"SELECT
		   id `id: PlayerID`,
		   name `name: PlayerName`,
		   ip_address `ip_address: std::net::Ipv4Addr`,
		   preferences `preferences: sqlx::types::Json<crate::players::Preferences>`,
		   created_at,
		   last_seen_at
		 FROM Players
		 WHERE name LIKE ?
		 ORDER BY created_at DESC
		 LIMIT ?
		 OFFSET ?",
		name,
		limit,
		offset,
	)
	.fetch(conn)
	.map_ok(|row| macros::map_row!(row));

	Ok(PaginationResults { total, stream })
}

pub struct GetPlayersParams<'a> {
	pub name: Option<&'a PlayerName>,
	pub limit: Limit,
	pub offset: Offset,
}

pub struct Player {
	pub id: PlayerID,
	pub name: PlayerName,
	pub ip_address: Ipv4Addr,
	pub preferences: Preferences,
	pub created_at: OffsetDateTime,
	pub last_seen_at: OffsetDateTime,
}

mod macros {
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

	pub(super) use map_row;
}
