//! The errors that can occur when interacting with this service.

use thiserror::Error;

use crate::runtime;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error {}

impl From<Error> for runtime::Error
{
	#[track_caller]
	fn from(value: Error) -> Self
	{
		match value {}
	}
}
