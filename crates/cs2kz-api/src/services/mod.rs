//! API services.
//!
//! Most of the API's business logic is split up into services.
//! Services will generally provide a constructor, and functions with the following signature:
//!
//! ```ignore
//! async fn(&self, request: Request) -> Result<Response, Error>;
//! ```
//!
//! If a service maps to an HTTP endpoint, it will also export an `http` module with a `router`
//! function returning an [`axum::Router`].

pub mod steam;
pub use steam::SteamService;

pub mod plugin;
pub use plugin::PluginService;

pub mod players;
pub use players::PlayerService;

pub mod servers;
pub use servers::ServerService;

pub mod maps;
pub use maps::MapService;

pub mod auth;
pub use auth::AuthService;
