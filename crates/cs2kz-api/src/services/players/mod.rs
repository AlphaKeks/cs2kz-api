//! A service to manage KZ players.

use derive_more::{Constructor, Debug};

use crate::database;
use crate::services::SteamService;

mod models;
pub use models::{PlayerIdentifier, PlayerInfo, Preferences};

pub mod register_player;
pub mod get_player;
pub mod get_players;
pub mod get_preferences;
pub mod update_player;
pub mod http;

/// A service to manage KZ players.
#[derive(Debug, Clone, Constructor)]
pub struct PlayerService
{
	#[debug("MySql")]
	mysql: database::Pool,
	steam_service: SteamService,
}
