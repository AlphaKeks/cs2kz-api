//! A middleware for catching panics.
//!
//! Normally, if an HTTP handler panics, the connection will simply be closed.
//! This middleware will catch panics and return a proper HTTP response.

use std::any::Any;

use axum::http;
use axum::response::IntoResponse;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

use crate::runtime;

/// Creates a middleware layer for catching panics and turning them into
/// responses.
pub fn layer() -> CatchPanicLayer<PanicHandler>
{
	CatchPanicLayer::custom(PanicHandler)
}

/// A custom panic handler for [`CatchPanicLayer`].
#[derive(Clone)]
pub struct PanicHandler;

impl ResponseForPanic for PanicHandler
{
	type ResponseBody = axum::body::Body;

	fn response_for_panic(
		&mut self,
		_err: Box<dyn Any + Send + 'static>,
	) -> http::Response<Self::ResponseBody>
	{
		runtime::Error::panic().into_response()
	}
}
