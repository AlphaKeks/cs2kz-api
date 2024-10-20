//! Custom [extractors].
//!
//! Most of these are wrappers around existing extractors that only alter the rejection responses.
//!
//! [extractors]: axum::extract

pub mod path;
pub use path::Path;

pub mod query;
pub use query::Query;

pub mod json;
pub use json::Json;
