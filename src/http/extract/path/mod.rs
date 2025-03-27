mod rejection;

use std::fmt;

use axum::{
	extract::{FromRequestParts, OptionalFromRequestParts},
	http::request,
};
use futures_util::TryFutureExt;
use serde::Deserialize;

pub(crate) use self::rejection::PathRejection;

#[derive(Debug)]
pub(crate) struct Path<T>(pub T)
where
	T: for<'de> Deserialize<'de>;

impl<T, S> FromRequestParts<S> for Path<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = PathRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		<axum::extract::Path<T> as FromRequestParts<S>>::from_request_parts(parts, state)
			.map_ok(|axum::extract::Path(value)| Self(value))
			.map_err(PathRejection::<T>::from)
			.await
	}
}

impl<T, S> OptionalFromRequestParts<S> for Path<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = PathRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Option<Self>, Self::Rejection>
	{
		<axum::extract::Path<T> as OptionalFromRequestParts<S>>::from_request_parts(parts, state)
			.map_ok(|maybe_path| maybe_path.map(|axum::extract::Path(value)| Self(value)))
			.map_err(PathRejection::<T>::from)
			.await
	}
}
