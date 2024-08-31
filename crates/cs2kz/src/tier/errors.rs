//! Error types used by the [`tier`] module.
//!
//! [`tier`]: crate::tier

use thiserror::Error;

/// Error produced when converting an integer or parsing a string into a [`Tier`].
///
/// [`Tier`]: super::Tier
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("invalid tier")]
pub struct InvalidTier;
