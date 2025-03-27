mod rejection;

use std::fmt;

use axum::{
	extract::{FromRequestParts, OptionalFromRequestParts},
	http::request,
};
use serde::Deserialize;

pub(crate) use self::rejection::QueryRejection;

#[derive(Debug)]
pub(crate) struct Query<T>(pub T)
where
	T: for<'de> Deserialize<'de>;

impl<T, S> FromRequestParts<S> for Query<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = QueryRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Self, Self::Rejection>
	{
		let query = parts.uri.query().unwrap_or_default();
		let value = serde_html_form::from_str(query)?;

		Ok(Self(value))
	}
}

impl<T, S> OptionalFromRequestParts<S> for Query<T>
where
	T: for<'de> Deserialize<'de> + fmt::Debug + Send + 'static,
	S: Send + Sync,
{
	type Rejection = QueryRejection<T>;

	#[tracing::instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Option<Self>, Self::Rejection>
	{
		let Some(query) = parts.uri.query().filter(|query| !query.is_empty()) else {
			return Ok(None);
		};

		serde_html_form::from_str(query)
			.map(Self)
			.map(Some)
			.map_err(QueryRejection::from)
	}
}
