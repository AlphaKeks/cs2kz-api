//! A middleware that wraps a [`tower::Service`] whose `Error` isn't infallible,
//! but implements [`IntoResponse`].
//!
//! This is useful because [`axum`], our web framework, only accepts infallible
//! services. [`InfallibleLayer`] allows configuring a service stack that will
//! always produce a response and can integrate with axum.

#![allow(unused)]

mod layer;
pub use layer::InfallibleLayer;

mod service;
pub use service::Infallible;

mod future;
pub use future::ResponseFuture;

#[cfg(test)]
mod tests
{
	use anyhow::Context;
	use axum::response::{IntoResponse, Response};
	use problem_details::AsProblemDetails;
	use tower::{service_fn, Layer, ServiceExt};

	use super::*;
	use crate::testing;

	#[tokio::test]
	async fn it_works() -> testing::Result
	{
		#[derive(Debug, Error)]
		#[error("whoops")]
		struct Whoops;

		impl IntoResponse for Whoops
		{
			fn into_response(self) -> Response
			{
				(http::StatusCode::IM_A_TEAPOT, "whoops").into_response()
			}
		}

		impl AsProblemDetails for Whoops
		{
			type ProblemType = crate::http::Problem;

			fn problem_type(&self) -> Self::ProblemType
			{
				crate::http::Problem::Whoops
			}
		}

		let request = http::Request::builder()
			.method(http::Method::GET)
			.uri("/")
			.body(crate::http::Body::default())?;

		let response = InfallibleLayer::new()
			.layer(service_fn(|_| async { Err(Whoops) }))
			.oneshot(request)
			.await?;

		testing::assert_eq!(response.status(), http::StatusCode::IM_A_TEAPOT);

		let error = testing::collect_res_body(response.into_body())
			.await
			.and_then(testing::deserialize_error)?;

		testing::assert_matches!(error.problem_type(), crate::http::Problem::Whoops);

		Ok(())
	}
}
