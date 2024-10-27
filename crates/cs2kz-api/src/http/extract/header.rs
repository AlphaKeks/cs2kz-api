//! The [`Header`] [extractor] and related types.
//!
//! [extractor]: axum::extract

use std::convert::Infallible;
use std::fmt;
use std::marker::PhantomData;

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use headers::Header as IsHeader;
use problem_details::AsProblemDetails;

use crate::http::problem_details::Problem;

/// An extractor for header values.
#[derive(Debug)]
pub struct Header<T>(pub T)
where
	T: IsHeader;

/// Rejection for the [`Header`] extractor.
pub enum HeaderRejection<T>
where
	T: IsHeader,
{
	/// There was no header of type `T`.
	Missing,

	/// We failed to decode the headers we found.
	DecodeValue(headers::Error),

	#[doc(hidden)]
	__Marker(PhantomData<fn() -> T>, Infallible),
}

impl<S, T> FromRequestParts<S> for Header<T>
where
	S: Send + Sync,
	T: IsHeader,
{
	type Rejection = HeaderRejection<T>;

	async fn from_request_parts(
		parts: &mut http::request::Parts,
		_: &S,
	) -> Result<Self, Self::Rejection> {
		let mut at_least_one_header = false;
		let mut headers = parts
			.headers
			.get(T::name())
			.into_iter()
			.inspect(|_| at_least_one_header = true);

		T::decode(&mut headers).map(Self).map_err(|error| {
			if at_least_one_header {
				HeaderRejection::DecodeValue(error)
			} else {
				HeaderRejection::Missing
			}
		})
	}
}

impl<T> fmt::Debug for HeaderRejection<T>
where
	T: IsHeader,
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		match *self {
			Self::Missing => write!(fmt, "Missing"),
			Self::DecodeValue(ref error) => fmt.debug_tuple("DecodeValue").field(error).finish(),
			Self::__Marker(_, never) => match never {},
		}
	}
}

impl<T> fmt::Display for HeaderRejection<T>
where
	T: IsHeader,
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		if is_auth_in_prod::<T>() {
			return write!(fmt, "you are not permitted to make this request");
		}

		match *self {
			Self::Missing => {
				write!(fmt, "missing `{}` header", T::name())
			},
			Self::DecodeValue(ref error) => {
				write!(fmt, "{error}")
			},
			Self::__Marker(_, never) => match never {},
		}
	}
}

impl<T> std::error::Error for HeaderRejection<T>
where
	T: IsHeader,
{
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match *self {
			Self::Missing => None,
			Self::DecodeValue(ref error) => Some(error),
			Self::__Marker(_, never) => match never {},
		}
	}
}

impl<T> AsProblemDetails for HeaderRejection<T>
where
	T: IsHeader,
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType {
		if is_auth_in_prod::<T>() {
			return Problem::Unauthorized;
		}

		match *self {
			Self::Missing => Problem::MissingHeader,
			Self::DecodeValue(_) => Problem::InvalidHeaderValue,
			Self::__Marker(_, never) => match never {},
		}
	}
}

impl<T> IntoResponse for HeaderRejection<T>
where
	T: IsHeader,
{
	fn into_response(self) -> Response {
		self.as_problem_details().into_response()
	}
}

fn is_auth_in_prod<H: IsHeader>() -> bool {
	H::name() == http::header::AUTHORIZATION && cfg!(feature = "production")
}
