use std::net::Ipv4Addr;

use crate::events::Event;
use crate::players::{PlayerID, PlayerName, Preferences};
use crate::{database, events, players};

#[instrument(skip(pool), err(level = "debug"))]
pub async fn register(pool: &database::ConnectionPool, player: NewPlayer) -> database::Result<()> {
	let mut txn = pool.begin().await?;

	players::database::create_or_update(&mut txn, players::database::NewPlayer {
		id: player.id,
		name: &player.name,
		ip_address: player.ip_address,
		preferences: &player.preferences,
	})
	.await?;

	txn.commit().await?;
	events::dispatch(Event::PlayerRegistered(player));

	Ok(())
}

#[derive(Debug, Clone)]
pub struct NewPlayer {
	pub id: PlayerID,
	pub name: PlayerName,
	pub ip_address: Ipv4Addr,
	pub preferences: Preferences,
}
