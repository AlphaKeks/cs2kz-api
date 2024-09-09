use derive_more::{Constructor, Debug};

use super::Infallible;

/// A layer producing the [`Infallible`] service.
///
/// # Example
///
/// ```
/// use axum::{routing, Router};
/// use cs2kz_api::middleware::InfallibleLayer;
/// use cs2kz_api::services::auth::session::SessionManagerLayer;
/// use cs2kz_api::services::AuthService;
/// use tower::ServiceBuilder;
///
/// fn foo(auth_svc: AuthService) -> Router
/// {
///     let stack = ServiceBuilder::new()
///         .layer(InfallibleLayer::new())
///         .layer(SessionManagerLayer::new(auth_svc)); // fallible!
///
///     Router::new()
///         .route("/", routing::get(|| async { "Hello, world!" }))
///         .route_layer(stack) // still works!
/// }
/// ```
#[derive(Debug, Constructor, Clone)]
pub struct InfallibleLayer {}

impl<S> tower::Layer<S> for InfallibleLayer
{
	type Service = Infallible<S>;

	fn layer(&self, inner: S) -> Self::Service
	{
		Infallible::new(inner)
	}
}
