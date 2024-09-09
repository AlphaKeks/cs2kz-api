//! This module contains the [`MapService`].

use derive_more::{Constructor, Debug};

use crate::database;
use crate::services::SteamService;

mod models;
pub use models::{Course, CourseID, Filter, FilterID, MapID, MapIdentifier, Mapper};

pub mod submit_map;
pub mod get_map;
pub mod get_maps;
pub mod update_map;
pub mod http;

/// A service to manage KZ maps.
#[derive(Debug, Clone, Constructor)]
pub struct MapService
{
	#[debug("MySql")]
	mysql: database::Pool,
	steam_service: SteamService,
}
