use std::{any::type_name, marker::PhantomData};

use axum::{
	extract::rejection::BytesRejection,
	response::{IntoResponse, Response},
};

use crate::http::problem_details::{ProblemDetails, ProblemType};

#[derive(Debug, Display)]
#[display("failed to extract json body of type `{}`: {}", type_name::<T>(), inner)]
pub(crate) struct JsonRejection<T>
{
	inner: JsonRejectionInner,

	#[debug("{}", type_name::<T>())]
	ty: PhantomData<T>,
}

#[derive(Debug, Display)]
enum JsonRejectionInner
{
	#[display("missing `Content-Type` header")]
	MissingContentType,
	BufferBody(BytesRejection),
	Deserialize(serde_json::Error),
}

impl<T> JsonRejection<T>
{
	pub(super) fn missing_content_type() -> Self
	{
		Self {
			inner: JsonRejectionInner::MissingContentType,
			ty: PhantomData,
		}
	}

	pub(super) fn deserialize(error: serde_json::Error) -> Self
	{
		Self {
			inner: JsonRejectionInner::Deserialize(error),
			ty: PhantomData,
		}
	}
}

impl<T> From<BytesRejection> for JsonRejection<T>
{
	fn from(rejection: BytesRejection) -> Self
	{
		Self {
			inner: JsonRejectionInner::BufferBody(rejection),
			ty: PhantomData,
		}
	}
}

impl<T> IntoResponse for JsonRejection<T>
{
	fn into_response(self) -> Response
	{
		match self.inner {
			JsonRejectionInner::MissingContentType => {
				let mut problem_details = ProblemDetails::new(ProblemType::MissingHeader);
				problem_details
					.add_extension_member("required_header", http::header::CONTENT_TYPE.as_str());
				problem_details.into_response()
			},
			JsonRejectionInner::BufferBody(rejection) => rejection.into_response(),
			JsonRejectionInner::Deserialize(error) => {
				let mut problem_details = ProblemDetails::new(ProblemType::DeserializeRequestBody);
				problem_details.set_detail(error.to_string());
				problem_details.into_response()
			},
		}
	}
}
