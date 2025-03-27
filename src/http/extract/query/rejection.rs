use std::{any::type_name, marker::PhantomData};

use axum::response::{IntoResponse, Response};

use crate::http::problem_details::{ProblemDetails, ProblemType};

#[derive(Debug, Display)]
#[display("failed to extract query parameter of type `{}`: {}", type_name::<T>(), kind)]
pub(crate) struct QueryRejection<T>
{
	kind: QueryRejectionKind,

	#[debug("{}", type_name::<T>())]
	ty: PhantomData<T>,
}

#[derive(Debug, Display)]
enum QueryRejectionKind
{
	Deserialize(serde_html_form::de::Error),
}

impl<T> From<serde_html_form::de::Error> for QueryRejection<T>
{
	fn from(error: serde_html_form::de::Error) -> Self
	{
		Self {
			kind: QueryRejectionKind::Deserialize(error),
			ty: PhantomData,
		}
	}
}

impl<T> IntoResponse for QueryRejection<T>
{
	fn into_response(self) -> Response
	{
		let mut problem_details = ProblemDetails::new(ProblemType::InvalidQueryParameters);

		match self.kind {
			QueryRejectionKind::Deserialize(ref error) => {
				problem_details.set_detail(error.to_string())
			},
		}

		problem_details.into_response()
	}
}
