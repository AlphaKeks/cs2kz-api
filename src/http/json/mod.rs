mod rejection;

use std::fmt;

use axum::{
	body::Bytes,
	extract::{FromRequest, Request},
	response::{IntoResponse, Response},
};
use headers::HeaderMapExt;
use mime::Mime;
use serde::{Deserialize, Serialize};

pub(crate) use self::rejection::JsonRejection;

#[derive(Debug)]
pub(crate) struct Json<T>(pub T);

impl<T: Serialize> IntoResponse for Json<T>
{
	fn into_response(self) -> Response
	{
		let body = serde_json::to_vec(&self.0).unwrap_or_else(|err| {
			panic!("failed to serialize response body: {err}");
		});

		let mut response = Response::new(body.into());
		response.headers_mut().typed_insert(headers::ContentType::json());
		response
	}
}

impl<T, S> FromRequest<S> for Json<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug,
	S: Send + Sync,
{
	type Rejection = JsonRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection>
	{
		if !has_json_content_type(req.headers()) {
			return Err(JsonRejection::missing_content_type());
		}

		let body = Bytes::from_request(req, state).await?;

		serde_json::from_slice(&body[..])
			.map(Self)
			.map_err(JsonRejection::deserialize)
	}
}

#[tracing::instrument(level = "trace", ret(level = "trace"))]
fn has_json_content_type(headers: &http::HeaderMap) -> bool
{
	let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
		tracing::debug!("request headers do not contain a `Content-Type` header");
		return false;
	};

	let Ok(content_type) = content_type.to_str() else {
		tracing::debug!("request headers contain a `Content-Type` header, but it's not UTF-8");
		return false;
	};

	let Ok(mime) = content_type.parse::<Mime>() else {
		tracing::debug!(
			"request headers contain a `Content-Type` header, but it's not a valid mime type"
		);
		return false;
	};

	mime.type_() == mime::APPLICATION
		&& (mime.subtype() == mime::JSON || mime.suffix() == Some(mime::JSON))
}
