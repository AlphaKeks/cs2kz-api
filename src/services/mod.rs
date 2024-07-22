//! API services.
//!
//! These contain the core business logic.
//!
//! If a service directly maps to an HTTP route, it will have an `http` module
//! and will implement `Into<axum::Router>`.

/* TODO:
 * - AntiCheat service
 *    - perhaps implement this as middleware?
 */

pub mod steam;
pub use steam::SteamService;

pub mod auth;
pub use auth::AuthService;

mod health;
pub use health::HealthService;

pub mod players;
pub use players::PlayerService;

pub mod maps;
pub use maps::MapService;

pub mod servers;
pub use servers::ServerService;

pub mod records;
pub use records::RecordService;

pub mod jumpstats;
pub use jumpstats::JumpstatService;

pub mod bans;
pub use bans::BanService;

pub mod admins;
pub use admins::AdminService;

mod plugin;
pub use plugin::PluginService;
