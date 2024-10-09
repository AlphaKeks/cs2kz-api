use std::net::Ipv4Addr;

use crate::database::{self, ErrorExt};
use crate::players::{PlayerID, PlayerName};

#[instrument(skip(conn), err(level = "debug"))]
pub async fn create(
	conn: &mut database::Connection,
	NewPlayer {
		id,
		name,
		ip_address,
	}: &NewPlayer,
) -> Result<(), CreatePlayerError> {
	sqlx::query! {
		"INSERT INTO Players (id, name, ip_address)
		 VALUES (?, ?, ?)",
		id,
		name,
		ip_address,
	}
	.execute(conn)
	.await
	.map_err(|error| {
		if error.is_unique_violation() {
			CreatePlayerError::DuplicatePlayerID
		} else {
			CreatePlayerError::Database(error)
		}
	})?;

	Ok(())
}

pub struct NewPlayer {
	pub id: PlayerID,
	pub name: PlayerName,
	pub ip_address: Ipv4Addr,
}

#[derive(Debug, Error)]
pub enum CreatePlayerError {
	#[error("duplicate player ID")]
	DuplicatePlayerID,

	#[error(transparent)]
	Database(#[from] database::Error),
}
