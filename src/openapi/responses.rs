//! Types implementing [`utoipa::IntoResponses`].

#![allow(clippy::missing_docs_in_private_items)]

use std::collections::BTreeMap;

use axum::response::{IntoResponse, Response};
use reqwest::StatusCode;
use serde::Serialize;
use utoipa::openapi::response::Response as ResponseSchema;
use utoipa::openapi::RefOr;
use utoipa::{IntoResponses, ToSchema};

#[derive(Debug, Serialize, IntoResponses)]
#[response(status = 200)]
pub struct Ok<T>(#[to_schema] pub T)
where
	T: ToSchema<'static>;

#[derive(Debug, Serialize)]
pub struct Created<T = ()>(pub T);

impl<T> IntoResponse for Created<T>
where
	T: IntoResponse,
{
	fn into_response(self) -> Response {
		(StatusCode::CREATED, self.0).into_response()
	}
}

impl<T> IntoResponses for Created<T>
where
	T: ToSchema<'static>,
{
	#[allow(clippy::missing_docs_in_private_items)]
	fn responses() -> BTreeMap<String, RefOr<ResponseSchema>> {
		#[derive(IntoResponses)]
		#[response(status = 201)]
		struct Helper<T>(#[to_schema] T)
		where
			T: ToSchema<'static>;

		Helper::<T>::responses()
	}
}

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 204)]
pub struct NoContent;

impl IntoResponse for NoContent {
	fn into_response(self) -> Response {
		StatusCode::NO_CONTENT.into_response()
	}
}

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 400)]
pub struct BadRequest;

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 401)]
pub struct Unauthorized;

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 409)]
pub struct Conflict;

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 422)]
pub struct UnprocessableEntity;

#[derive(Debug, Clone, Copy, Serialize, IntoResponses)]
#[response(status = 502)]
pub struct BadGateway;

/// A generic JSON object.
#[derive(Debug, Clone, Copy, ToSchema)]
#[schema(value_type = serde_json::Value)]
pub struct JsonObject;
