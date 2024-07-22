//! Various middlewares.

pub mod logging;
pub mod panic_handler;

mod infallible;
pub use infallible::InfallibleLayer;
