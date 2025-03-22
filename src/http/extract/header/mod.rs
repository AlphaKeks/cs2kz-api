mod rejection;

use std::fmt;

use axum::{
	extract::{FromRequestParts, OptionalFromRequestParts},
	http::request,
};
use headers::HeaderMapExt;

pub(crate) use self::rejection::HeaderRejection;

#[derive(Debug)]
pub(crate) struct Header<T>(pub T)
where
	T: headers::Header;

impl<T, S> FromRequestParts<S> for Header<T>
where
	T: headers::Header + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = HeaderRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Self, Self::Rejection>
	{
		parts
			.headers
			.typed_try_get::<T>()?
			.map(Self)
			.ok_or_else(HeaderRejection::missing)
	}
}

impl<T, S> OptionalFromRequestParts<S> for Header<T>
where
	T: headers::Header + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = HeaderRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Option<Self>, Self::Rejection>
	{
		parts
			.headers
			.typed_try_get::<T>()
			.map(|maybe_header| maybe_header.map(Self))
			.map_err(HeaderRejection::from)
	}
}
