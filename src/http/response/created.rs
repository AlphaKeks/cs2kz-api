use {
	crate::http::json::Json,
	axum::response::{IntoResponse, Response},
	serde::Serialize,
};

#[derive(Debug)]
pub(crate) struct Created<T>(pub T)
where
	T: Serialize;

impl<T: Serialize> IntoResponse for Created<T>
{
	fn into_response(self) -> Response
	{
		(http::StatusCode::CREATED, Json(self.0)).into_response()
	}
}
