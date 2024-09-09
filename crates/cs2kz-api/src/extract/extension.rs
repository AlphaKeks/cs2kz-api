#![allow(clippy::disallowed_types)]

use axum::extract::FromRequestParts;
use problem_details::AsProblemDetails;

use crate::http::Problem;

mod base
{
	pub use axum::extract::rejection::ExtensionRejection as Rejection;
	pub use axum::middleware::AddExtension;
	pub use axum::Extension as Extractor;
}

/// An [extractor] for [request extensions].
///
/// This is the same as [`axum::extract::Extension`], except that it produces the same kind of
/// error response as all of our errors.
///
/// [extractor]: axum::extract
/// [request extensions]: http::Request::extensions
#[derive(Debug, Clone, FromRequestParts)]
#[from_request(via(base::Extractor), rejection(ExtensionRejection))]
pub struct Extension<T>(pub T);

impl<T, S> tower::Layer<S> for Extension<T>
where
	T: Clone + Send + Sync + 'static,
{
	type Service = base::AddExtension<S, T>;

	fn layer(&self, inner: S) -> Self::Service
	{
		base::Extractor(self.0.clone()).layer(inner)
	}
}

/// Rejection for the [`Extension`] extractor.
#[derive(Debug, Error)]
#[error(transparent)]
pub struct ExtensionRejection(#[from] base::Rejection);

impl AsProblemDetails for ExtensionRejection
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		Problem::Internal
	}
}

impl_into_response!(ExtensionRejection);
