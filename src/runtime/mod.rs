//! Anything related to the API's runtime - nothing specific business logic.

pub mod panic_hook;
pub(crate) mod signals;

mod error;
pub(crate) use error::{Error, Result};

mod config;
pub use config::Config;
