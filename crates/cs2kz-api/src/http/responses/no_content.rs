use axum::response::{IntoResponse, Response};

/// A `204 No Content` response.
#[derive(Debug, Clone, Copy)]
pub struct NoContent;

impl IntoResponse for NoContent {
	fn into_response(self) -> Response {
		http::StatusCode::NO_CONTENT.into_response()
	}
}
