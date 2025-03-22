use std::{any::type_name, error::Error, marker::PhantomData};

use axum::response::{IntoResponse, Response};

use crate::http::problem_details::{ProblemDetails, ProblemType};

#[derive(Debug, Display)]
#[display("failed to extract path parameter of type `{}`: {}", type_name::<T>(), inner)]
pub(crate) struct PathRejection<T>
{
	inner: axum::extract::rejection::PathRejection,

	#[debug("{}", type_name::<T>())]
	ty: PhantomData<T>,
}

impl<T> From<axum::extract::rejection::PathRejection> for PathRejection<T>
{
	fn from(rejection: axum::extract::rejection::PathRejection) -> Self
	{
		Self { inner: rejection, ty: PhantomData }
	}
}

impl<T> IntoResponse for PathRejection<T>
{
	fn into_response(self) -> Response
	{
		use axum::extract::path::ErrorKind;

		let error = match self.inner {
			axum::extract::rejection::PathRejection::FailedToDeserializePathParams(error) => error,
			error @ (axum::extract::rejection::PathRejection::MissingPathParams(_) | _) => {
				tracing::error!(
					error = &error as &dyn Error,
					"type" = type_name::<T>(),
					"failed to deserialize path parameter(s)",
				);

				return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
			},
		};

		let mut problem_details = ProblemDetails::new(ProblemType::InvalidPathParameters);

		match error.kind() {
			ErrorKind::WrongNumberOfParameters { got, expected } => {
				tracing::error!(
					error = &error as &dyn Error,
					got,
					expected,
					"type" = type_name::<T>(),
					"attempted to deserialize unexpected amount of path parameters",
				);

				return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
			},
			ErrorKind::ParseErrorAtKey { key, value, expected_type } => {
				problem_details.set_detail(format!(
					"failed to parse {key:?} parameter: {value:?} is not a valid `{expected_type}`",
				));
			},
			ErrorKind::ParseErrorAtIndex { index, value, expected_type } => {
				problem_details.set_detail(format!(
					"failed to parse parameter #{index}: {value:?} is not a valid \
					 `{expected_type}`",
				));
			},
			ErrorKind::ParseError { value, expected_type } => {
				problem_details.set_detail(format!(
					"failed to parse parameter: {value:?} is not a valid `{expected_type}`",
				));
			},
			ErrorKind::InvalidUtf8InPathParam { key } => {
				problem_details
					.set_detail(format!("failed to parse parameter {key:?}: invalid UTF-8"));
			},
			ErrorKind::UnsupportedType { name } => {
				tracing::error!(
					error = &error as &dyn Error,
					name,
					"type" = type_name::<T>(),
					"attempted to deserialize unsupported type",
				);

				return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
			},
			ErrorKind::DeserializeError { key, message, .. } => {
				problem_details
					.set_detail(format!("failed to parse parameter {key:?}: {message}",));
			},
			ErrorKind::Message(message) => {
				tracing::error!(
					error = &error as &dyn Error,
					"type" = type_name::<T>(),
					"{message}",
				);

				return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
			},
			_ => {
				tracing::error!(
					error = &error as &dyn Error,
					"type" = type_name::<T>(),
					"failed to deserialize path parameter(s)"
				);

				return http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
			},
		}

		problem_details.into_response()
	}
}
