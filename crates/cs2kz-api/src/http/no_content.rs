use axum::response::{IntoResponse, Response};
use derive_more::Constructor;

use super::Location;
use crate::extract::Json;

#[derive(Debug, Clone)]
pub struct NoContent;

impl IntoResponse for NoContent
{
	fn into_response(self) -> Response
	{
		http::StatusCode::NO_CONTENT.into_response()
	}
}
