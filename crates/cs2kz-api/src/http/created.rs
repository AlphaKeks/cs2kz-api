use axum::response::{IntoResponse, Response};
use derive_more::Constructor;

use super::Location;
use crate::extract::Json;

#[derive(Debug, Clone, Constructor)]
pub struct Created<T>
{
	location: Location,
	payload: T,
}

impl<T> IntoResponse for Created<T>
where
	Json<T>: IntoResponse,
{
	fn into_response(self) -> Response
	{
		(http::StatusCode::CREATED, self.location, Json(self.payload)).into_response()
	}
}
