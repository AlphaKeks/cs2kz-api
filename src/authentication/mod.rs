//! Everything related to authentication.

mod jwt;

#[doc(inline)]
pub use jwt::Jwt;

pub mod session;

#[doc(inline)]
pub use session::Session;

pub mod api_key;

#[doc(inline)]
pub use api_key::ApiKey;

mod user;

#[doc(inline)]
pub use user::User;

pub mod server;

#[doc(inline)]
pub use server::Server;
