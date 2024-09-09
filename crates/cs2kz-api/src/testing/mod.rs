//! Utilities for unit & integration tests.

use anyhow::Context as _;
use serde::Deserialize;

mod macros;

#[expect(unused_imports, reason = "these may be used later")]
pub(crate) use macros::*;

pub type Error = anyhow::Error;
pub type Result<T = (), E = Error> = std::result::Result<T, E>;

pub async fn collect_res_body(body: crate::http::Body) -> Result<axum::body::Bytes>
{
	axum::body::to_bytes(body, usize::MAX)
		.await
		.context("collect response body")
}

#[track_caller]
pub fn deserialize_body<T>(body: impl AsRef<[u8]>) -> Result<T>
where
	T: for<'de> Deserialize<'de>,
{
	serde_json::from_slice::<T>(body.as_ref()).context("deserialize response body as error")
}

#[track_caller]
pub fn deserialize_error(body: impl AsRef<[u8]>) -> Result<crate::http::ProblemDetails>
{
	deserialize_body::<crate::http::ProblemDetails>(body)
		.context("deserialize response body as error")
}

#[ctor::ctor]
fn ctor()
{
	crate::http::problem::set_base_uri(
		"https://docs.cs2kz.org/api/problems"
			.parse()
			.expect("valid uri"),
	);
}
