//! User session management.
//!
//! Users authenticate via [Steam] and receive a session token upon login. This token will be
//! extended everytime they make an authenticated request, and can be invalidated explicitly.
//!
//! [Steam]: https://steamcommunity.com

mod id;
pub use id::SessionID;

mod session;
pub use session::Session;

pub mod authorization;

pub(super) mod http;
mod database;
