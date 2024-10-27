//! The [`Json`] [extractor] and related types.
//!
//! [extractor]: axum::extract

use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use mime::Mime;
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;

/// An extractor for JSON request bodies.
///
/// This type also implements [`IntoResponse`], which means it can be returned
/// from handlers.
///
/// See [`axum::Json`] for more details.
#[derive(Debug)]
pub struct Json<T>(pub T);

/// Rejection for the [`Json`] extractor.
#[derive(Debug, Error)]
#[expect(missing_docs)]
pub enum JsonRejection {
	#[error("missing `Content-Type: application/json` header")]
	MissingJsonContentType,

	#[error("failed to buffer request body")]
	BufferRequestBody(#[from] BytesRejection),

	#[error("failed to deserialize request body: {0}")]
	DeserializeRequestBody(#[from] json::Error),
}

impl<S, T> FromRequest<S> for Json<T>
where
	S: Send + Sync,
	T: for<'de> serde::Deserialize<'de>,
{
	type Rejection = JsonRejection;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		if !has_json_content_type(req.headers()) {
			return Err(JsonRejection::MissingJsonContentType);
		}

		let bytes = Bytes::from_request(req, state).await?;
		let value = json::from_slice::<T>(&bytes)?;

		Ok(Self(value))
	}
}

impl<T> IntoResponse for Json<T>
where
	T: serde::Serialize,
{
	fn into_response(self) -> Response {
		axum::Json(self.0).into_response()
	}
}

impl AsProblemDetails for JsonRejection {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		match self {
			Self::MissingJsonContentType => Problem::MissingHeader,
			Self::BufferRequestBody(_) => Problem::BadRequest,
			Self::DeserializeRequestBody(_) => Problem::DeserializeRequestBody,
		}
	}

	fn add_extension_members(&self, extension_members: &mut problem_details::ExtensionMembers) {
		match self {
			Self::MissingJsonContentType => {
				_ = extension_members.add("header_name", http::header::CONTENT_TYPE.as_str());
				_ = extension_members.add("header_value", mime::APPLICATION_JSON.as_ref());
			},
			Self::BufferRequestBody(_) => {},
			Self::DeserializeRequestBody(error) => {
				_ = extension_members.add("line", &error.line());
				_ = extension_members.add("column", &error.column());
			},
		}
	}
}

impl IntoResponse for JsonRejection {
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

/// Checks if the given `headers` contain a `Content-Type` header with a
/// JSON-related value.
#[instrument(level = "trace", ret(level = "trace"))]
fn has_json_content_type(headers: &http::HeaderMap) -> bool {
	let Some(content_type) = headers.get(http::header::CONTENT_TYPE) else {
		return false;
	};

	let Ok(content_type) = content_type.to_str() else {
		return false;
	};

	let Ok(mime) = content_type.parse::<Mime>() else {
		return false;
	};

	mime.type_() == "application"
		&& (mime.subtype() == "json" || mime.suffix().map_or(false, |name| name == "json"))
}
