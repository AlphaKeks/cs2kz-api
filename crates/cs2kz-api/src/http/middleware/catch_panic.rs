//! A middleware to catch panics and turn them into responses.
//!
//! See [`tower_http::catch_panic`] for more details.

use std::any::Any;
use std::marker::PhantomData;
use std::sync::LazyLock;

use bytes::Bytes;
use problem_details::AsProblemDetails;
use tower_http::catch_panic::{CatchPanicLayer, ResponseForPanic};

use crate::http::problem_details::Problem;

/// The response we want to return in case of a panic.
static RESPONSE: LazyLock<http::Response<Bytes>> = LazyLock::new(|| {
	#[derive(Debug, Error)]
	#[error("something unexpected happened; please report this incident")]
	struct Panic;

	impl AsProblemDetails for Panic {
		type ProblemType = Problem;

		fn problem_type(&self) -> Self::ProblemType {
			Problem::Internal
		}
	}

	Panic.as_problem_details().into()
});

/// Creates a [`tower::Layer`], which produces a middleware that will catch
/// panics in its inner service and turn them into HTTP responses.
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
			.map(String::as_str)
			.or_else(|| err.downcast_ref::<&str>().copied());

		error!(?panic_message, "http handler panicked");

		// Backtraces are expensive, so only generate those when running locally.
		#[cfg(not(feature = "production"))]
		{
			error!(backtrace = %std::backtrace::Backtrace::force_capture());
		}

		RESPONSE.clone().map(B::from)
	}
}
