//! Error types used by the [`ranked_status`] module.
//!
//! [`ranked_status`]: crate::ranked_status

use thiserror::Error;

/// Error produced when converting an integer or parsing a string into a [`RankedStatus`].
///
/// [`RankedStatus`]: super::RankedStatus
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("invalid ranked status")]
pub struct InvalidRankedStatus;
