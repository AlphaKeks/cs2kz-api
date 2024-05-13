//! Authentication / Authorization middleware using the [`Session`] extractor.
//!
//! [`Session`]: crate::authentication::Session

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::authentication;
use crate::authorization::AuthorizeSession;

/// Authenticates the incoming request and extends its session.
pub async fn layer<Authorization>(
	session: authentication::Session<Authorization>,
	mut request: Request,
	next: Next,
) -> (authentication::Session<Authorization>, Response)
where
	Authorization: AuthorizeSession,
{
	request.extensions_mut().insert(session.clone());

	(session, next.run(request).await)
}

/// macro
macro_rules! session_auth {
	($state:expr, $authorization:ty) => {
		|| {
			::axum::middleware::from_fn_with_state(
				$state,
				$crate::http::middleware::auth::layer::<$authorization>,
			)
		}
	};
}

pub(crate) use session_auth;
