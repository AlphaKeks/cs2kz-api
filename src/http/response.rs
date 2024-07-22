//! HTTP response types.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// An empty response with status `204 No Content`.
pub struct NoContent;

impl From<NoContent> for StatusCode
{
	fn from(_: NoContent) -> Self
	{
		Self::NO_CONTENT
	}
}

impl IntoResponse for NoContent
{
	fn into_response(self) -> Response
	{
		StatusCode::from(self).into_response()
	}
}
