use std::convert::Infallible;

use axum::response::{AppendHeaders, IntoResponse, IntoResponseParts, ResponseParts};

use super::Response;

#[derive(Debug, Clone)]
pub struct Location(http::HeaderValue);

impl From<http::HeaderValue> for Location
{
	fn from(value: http::HeaderValue) -> Self
	{
		Self(value)
	}
}

impl IntoResponseParts for Location
{
	type Error =
		<AppendHeaders<[(http::HeaderName, http::HeaderValue); 1]> as IntoResponseParts>::Error;

	fn into_response_parts(self, response: ResponseParts) -> Result<ResponseParts, Self::Error>
	{
		AppendHeaders([(http::header::LOCATION, self.0)]).into_response_parts(response)
	}
}
