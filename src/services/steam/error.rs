//! The errors that can occur when interacting with this service.

use std::io;

use axum::response::{IntoResponse, Response};
use thiserror::Error;

use super::WorkshopID;
use crate::runtime;

/// Type alias with a default `Err` type of [`Error`].
///
/// [`Error`]: enum@Error
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// The errors that can occur when interacting with the map service.
#[derive(Debug, Error)]
pub enum Error
{
	/// Extracting an OpenID payload failed.
	#[error(transparent)]
	MissingOpenIDPayload(#[from] axum_extra::extract::QueryRejection),

	/// An OpenID payload we sent to Steam for verification came back as
	/// invalid.
	#[error("failed to verify openid payload with Steam")]
	VerifyOpenIDPayload,

	/// Steam's API returned an error when we tried to fetch the map with this
	/// ID.
	#[error("invalid workshop ID `{workshop_id}`")]
	InvalidWorkshopID
	{
		workshop_id: WorkshopID
	},

	/// We failed to download a workshop map.
	#[error("failed to download workshop map: {0}")]
	DownloadWorkshopMap(io::Error),

	/// Calling out to Steam's API failed.
	#[error("failed to make http request: {0}")]
	Http(#[from] reqwest::Error),
}

impl IntoResponse for Error
{
	fn into_response(self) -> Response
	{
		runtime::Error::from(self).into_response()
	}
}

impl From<Error> for runtime::Error
{
	#[track_caller]
	fn from(value: Error) -> Self
	{
		match value {
			_ => todo!(),
		}
	}
}
