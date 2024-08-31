//! Error types used by the [`jump_type`] module.
//!
//! [`jump_type`]: crate::jump_type

use thiserror::Error;

/// Error produced when converting an integer or parsing a string into a [`JumpType`].
///
/// [`JumpType`]: super::JumpType
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("invalid jump type")]
pub struct InvalidJumpType;
