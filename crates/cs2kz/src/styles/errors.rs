//! Error types used by the [`styles`] module.
//!
//! [`styles`]: crate::styles

use thiserror::Error;

/// Error returned by a [`Styles`] constructor, indicating an integer with an invalid bit set to
/// 1 was passed.
///
/// [`Styles`]: super::Styles
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("invalid style bit")]
pub struct InvalidBit;
