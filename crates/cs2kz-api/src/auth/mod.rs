pub mod error;
pub use error::{Error, Result};

mod id;
pub use id::SessionID;

mod permissions;
pub use permissions::Permissions;

mod data;
pub use data::SessionData;

mod store;
pub use store::SessionStore;

pub mod authorization;

pub type Session = tower_sessions::Session<SessionStore>;
