use axum::{extract::Request, middleware::Next, response::Response};

use crate::{
	http::response::{HandlerError, HandlerResult},
	runtime::{self, Environment},
};

/// Middleware to check if a given request is coming from localhost.
///
/// This is used for private endpoints like `/metrics` and `/taskdump`.
#[tracing::instrument(skip_all, err(Debug, level = "debug"))]
pub(crate) async fn is_localhost(request: Request, next: Next) -> HandlerResult<Response>
{
	match runtime::environment::get() {
		Environment::Development => Ok(next.run(request).await),
		Environment::Testing | Environment::Production => {
			// If there's an X-Real-Ip header, we went through nginx, which
			// means the request came from the outside.
			if request.headers().contains_key("X-Real-Ip") {
				Err(HandlerError::NotFound)
			} else {
				Ok(next.run(request).await)
			}
		},
	}
}
