//! Session authentication with [`tower`].

#![allow(warnings)]

mod errors;
pub use errors::SessionManagerError;

mod strict;
pub use strict::Strict;

mod cookie_options;
pub use cookie_options::CookieOptions;

mod id;
pub use id::SessionID;

mod session;
pub use session::Session;

pub mod authorization;
pub use authorization::AuthorizeSession;

mod store;
pub use store::SessionStore;

mod layer;
pub use layer::SessionManagerLayer;

mod service;
pub use service::SessionManager;
