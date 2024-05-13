//! Errors that can occur when interacting with Steam's Workshop API.

use thiserror::Error;

use crate::http::HandlerError;

/// The different kinds of errors that can occurr when interacting with the Steam Workshop.
#[derive(Debug, Error)]
pub enum WorkshopError {
	/// A request failed because an invalid [WorkshopID] was passed.
	#[error("invalid workshop ID")]
	InvalidWorkshopID,

	/// A request failed.
	#[error("failed to fetch workshop info: {0}")]
	Http(#[from] reqwest::Error),

	/// Deserializing a response from Steam failed.
	#[error("failed to deserialize response from steam: {0}")]
	Deserialize(#[from] serde_json::Error),
}

impl From<WorkshopError> for HandlerError {
	fn from(error: WorkshopError) -> Self {
		let message = error.to_string();

		match error {
			WorkshopError::InvalidWorkshopID => Self::bad_request().with_message(message),
			WorkshopError::Http(err) => Self::internal_server_error()
				.with_message(message)
				.with_source(err),
			WorkshopError::Deserialize(err) => Self::internal_server_error()
				.with_message(message)
				.with_source(err),
		}
	}
}
