//! Error types used by the [`mode`] module.
//!
//! [`mode`]: crate::mode

use thiserror::Error;

/// Error returned from a `u8 -> Mode` conversion.
///
/// Only two `u8`s are valid [`Mode`]s: `1` and `2`.
///
/// [`Mode`]: super::Mode
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("integer does not correspond to a known mode")]
pub struct TryFromIntError;

/// Error returned when parsing a string into a [`Mode`].
///
/// [`Mode`]: super::Mode
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("unknown mode")]
pub struct ParseModeError;
