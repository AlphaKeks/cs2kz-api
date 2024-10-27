use std::sync::{Arc, LazyLock};

use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::{Router, routing};
use bytes::Bytes;
use problem_details::AsProblemDetails;
use tower_http::services::redirect::Redirect;
use utoipa_swagger_ui::SwaggerFile;

use crate::http::problem_details::Problem;

pub fn router<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	let swagger_ui_handler =
		routing::get(serve_swagger_ui).with_state(Arc::new(crate::openapi::swagger_ui_config()));

	let redirect_swagger_ui =
		Redirect::<axum::body::Body>::permanent(http::Uri::from_static("/docs/swagger-ui/"));

	Router::new()
		.route("/openapi.json", routing::get(serve_openapi_doc))
		.route_service("/swagger-ui", routing::get_service(redirect_swagger_ui))
		.route("/swagger-ui/", swagger_ui_handler.clone())
		.route("/swagger-ui/{*rest}", swagger_ui_handler)
}

/// Serves the API's OpenAPI schema as a JSON document.
async fn serve_openapi_doc() -> impl IntoResponse {
	// We only want to generate and serialize the schema once.
	//
	// `LazyLock` will do this automatically on first access, and from there on the
	// result is cached.
	static APIDOC: LazyLock<Bytes> = LazyLock::new(|| {
		json::to_vec(crate::openapi::Schema::new().api_doc())
			.expect("OpenAPI schema should be valid JSON")
			.into()
	});

	// `Bytes` does reference counting, so this is cheap.
	Bytes::clone(&APIDOC)
}

enum SwaggerUIResponse {
	NotFound,
	Error(ServeSwaggerFileError),
	Success(SwaggerFile<'static>),
}

#[derive(Debug, Error)]
#[error("something went wrong; please report this incident")]
struct ServeSwaggerFileError(#[source] Box<dyn std::error::Error>);

impl AsProblemDetails for ServeSwaggerFileError {
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		Problem::Internal
	}
}

impl IntoResponse for SwaggerUIResponse {
	fn into_response(self) -> Response {
		match self {
			Self::NotFound => http::StatusCode::NOT_FOUND.into_response(),
			Self::Error(error) => error.as_problem_details().into_response(),
			Self::Success(SwaggerFile {
				bytes,
				content_type,
				..
			}) => Response::builder()
				.status(http::StatusCode::OK)
				.header(http::header::CONTENT_TYPE, content_type)
				.body(bytes.into())
				.expect("utoipa should produce valid `Content-Type`"),
		}
	}
}

/// Serves SwaggerUI files.
async fn serve_swagger_ui(
	State(swagger_ui_config): State<Arc<utoipa_swagger_ui::Config<'static>>>,
	path: Option<Path<String>>,
) -> SwaggerUIResponse {
	let tail = match path {
		None => "",
		Some(Path(ref path)) => path.as_str(),
	};

	match utoipa_swagger_ui::serve(tail, swagger_ui_config) {
		Err(error) => {
			error!(%error, "failed to serve SwaggerUI");
			SwaggerUIResponse::Error(ServeSwaggerFileError(error))
		},
		Ok(None) => SwaggerUIResponse::NotFound,
		Ok(Some(file)) => SwaggerUIResponse::Success(file),
	}
}
