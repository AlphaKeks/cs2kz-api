//! A middleware to catch panics and turn them into responses.
//!
//! See [`tower_http::catch_panic`] for more details.

use std::any::Any;
use std::marker::PhantomData;

use bytes::Bytes;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

static RESPONSE_BODY: Bytes =
	Bytes::from_static(b"something unexpected happened; please report this incident");

/// Creates a [`tower::Layer`], which produces a middleware that will catch panics in its inner
/// service and turn them into HTTP responses.
pub fn layer<B>() -> CatchPanicLayer<impl ResponseForPanic<ResponseBody = B>>
where
	B: From<Bytes>,
{
	CatchPanicLayer::custom(PanicResponse(PhantomData))
}

#[derive(Copy)]
struct PanicResponse<B>(PhantomData<fn() -> B>);

// `#[derive(Clone)]` would generate a `B: Clone` bound, which is not necessary.
impl<B> Clone for PanicResponse<B> {
	fn clone(&self) -> Self {
		Self(PhantomData)
	}
}

impl<B> ResponseForPanic for PanicResponse<B>
where
	B: From<Bytes>,
{
	type ResponseBody = B;

	fn response_for_panic(
		&mut self,
		err: Box<dyn Any + Send + 'static>,
	) -> http::Response<Self::ResponseBody> {
		let panic_message = err
			.downcast_ref::<String>()
			.map(|s| s.as_str())
			.or_else(|| err.downcast_ref::<&str>().copied());

		error!(?panic_message, "http handler panicked");

		#[cfg(not(feature = "production"))]
		{
			error!(backtrace = %std::backtrace::Backtrace::force_capture());
		}

		http::Response::builder()
			.status(http::StatusCode::INTERNAL_SERVER_ERROR)
			.body(B::from(RESPONSE_BODY.clone()))
			.unwrap()
	}
}
