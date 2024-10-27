use axum::response::{IntoResponse, Response};

use crate::http::extract::Json;

/// A `201 Created` response.
#[derive(Debug)]
pub struct Created<T>(pub T);

impl<T> IntoResponse for Created<T>
where
	Json<T>: IntoResponse,
{
	fn into_response(self) -> Response {
		(http::StatusCode::CREATED, Json(self.0)).into_response()
	}
}
