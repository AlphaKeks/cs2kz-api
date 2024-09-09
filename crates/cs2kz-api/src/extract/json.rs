#![allow(clippy::disallowed_types)]

use axum::async_trait;
use axum::body::Bytes;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use problem_details::AsProblemDetails;
use serde::de::DeserializeOwned;

use crate::http::Problem;

mod base
{
	pub use axum::Json as Extractor;
}

/// An [extractor] for JSON request bodies.
///
/// This is the same as [`axum::Json`], except that it produces the same kind of error response as
/// all of our errors.
///
/// [extractor]: axum::extract
/// [request extensions]: http::Request::extensions
#[derive(Debug)]
pub struct Json<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for Json<T>
where
	S: Send + Sync,
	T: DeserializeOwned,
{
	type Rejection = JsonRejection;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection>
	{
		if !has_json_content_type(req.headers()) {
			return Err(JsonRejection::MissingContentType);
		}

		let bytes = Bytes::from_request(req, state).await?;
		let parsed = serde_json::from_slice(&bytes)?;

		Ok(Self(parsed))
	}
}

impl<T> IntoResponse for Json<T>
where
	base::Extractor<T>: IntoResponse,
{
	fn into_response(self) -> Response
	{
		base::Extractor(self.0).into_response()
	}
}

/// Rejection for the [`Json`] extractor.
#[derive(Debug, Error)]
pub enum JsonRejection
{
	#[error("missing `Content-Type: application/json` header")]
	MissingContentType,

	#[error("failed to read request body")]
	ReadRequestBody(#[from] axum::extract::rejection::BytesRejection),

	#[error(transparent)]
	Deserialize(#[from] serde_json::Error),
}

impl AsProblemDetails for JsonRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		use serde_json::error::Category as ECategory;

		match self {
			Self::MissingContentType => Problem::MissingHeader,
			Self::ReadRequestBody(_) => Problem::InvalidRequestBody,
			Self::Deserialize(source) => match source.classify() {
				ECategory::Io => unreachable!(),
				ECategory::Syntax | ECategory::Data | ECategory::Eof => Problem::InvalidRequestBody,
			},
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers)
	{
		if let Self::Deserialize(source) = self {
			extension_members.add("line", &source.line());
			extension_members.add("column", &source.column());
		}
	}
}

impl_into_response!(JsonRejection);

/// Checks if the given `headers` contain a JSON-like Content-Type.
fn has_json_content_type(headers: &http::HeaderMap) -> bool
{
	let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
		return false;
	};

	let Ok(content_type) = content_type.to_str() else {
		return false;
	};

	let Ok(mime) = content_type.parse::<mime::Mime>() else {
		return false;
	};

	mime.type_() == "application"
		&& (mime.subtype() == "json" || mime.suffix().is_some_and(|name| name == "json"))
}
