//! Functions to interact with the `Players` table.

mod create;
pub use create::{create_or_update, NewPlayer};

mod get;
pub use get::{get, get_by_id, get_by_name, GetPlayersParams, Player};
