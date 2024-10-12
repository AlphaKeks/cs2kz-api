use std::net::Ipv4Addr;

use crate::database::{self, ErrorExt};
use crate::players::{PlayerID, PlayerName, Preferences};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn create_or_update(
	conn: &mut database::Connection,
	NewPlayer {
		id,
		name,
		ip_address,
		preferences,
	}: NewPlayer<'_>,
) -> database::Result<()> {
	sqlx::query!(
		"INSERT INTO Players (
		   id,
		   name,
		   ip_address,
		   preferences
		 )
		 VALUES (?, ?, ?, ?)
		 ON DUPLICATE KEY
		 UPDATE name = VALUE(name),
		        ip_address = VALUE(ip_address),
		        preferences = VALUE(preferences),
		        last_seen_at = NOW()",
		id,
		name,
		ip_address,
		sqlx::types::Json(preferences),
	)
	.execute(conn)
	.await?;

	Ok(())
}

pub struct NewPlayer<'a> {
	pub id: PlayerID,
	pub name: &'a PlayerName,
	pub ip_address: Ipv4Addr,
	pub preferences: &'a Preferences,
}
