//! Error types used by the [`map_status`] module.
//!
//! [`map_status`]: crate::map_status

use thiserror::Error;

/// Error produced when converting an integer or parsing a string into a [`MapState`].
///
/// [`MapState`]: super::MapState
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("invalid map state")]
pub struct InvalidMapState;
