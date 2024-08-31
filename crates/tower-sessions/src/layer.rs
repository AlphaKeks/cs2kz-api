use std::sync::Arc;

use tower_layer::Layer;
use tower_service::Service;

use crate::{authorization, AuthorizeSession, CookieOptions, SessionManager, SessionStore, Strict};

/// A [`Layer`] producing [`SessionManager`]s.
#[derive(Debug, Clone)]
pub struct SessionManagerLayer<Store, ReqBody, Auth = authorization::None<ReqBody, Store>>
where
	Store: SessionStore,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
{
	/// How strict we should be when checking requests.
	strict: Strict,

	/// Values to use when constructing cookies.
	cookie_options: Arc<CookieOptions>,

	/// The authorization strategy.
	authorization: Auth,

	/// The session store.
	store: Store,
}

impl<Store, ReqBody> SessionManagerLayer<Store, ReqBody>
where
	Store: SessionStore,
	ReqBody: Send + 'static,
{
	/// Constructs a new [`SessionManagerLayer`].
	pub fn new(strict: Strict, cookie_options: impl Into<Arc<CookieOptions>>, store: Store)
		-> Self
	{
		Self {
			strict,
			cookie_options: cookie_options.into(),
			authorization: authorization::None::new(),
			store,
		}
	}
}

impl<Store, ReqBody, Auth> SessionManagerLayer<Store, ReqBody, Auth>
where
	Store: SessionStore,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
{
	/// Sets the authorization strategy.
	pub fn with_authorization<NewAuth>(
		self,
		authorization: NewAuth,
	) -> SessionManagerLayer<Store, ReqBody, NewAuth>
	where
		NewAuth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	{
		SessionManagerLayer {
			strict: self.strict,
			cookie_options: self.cookie_options,
			authorization,
			store: self.store,
		}
	}
}

impl<Inner, Store, ReqBody, Auth> Layer<Inner> for SessionManagerLayer<Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>>,
	Store: SessionStore + Clone,
	ReqBody: Send + 'static,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store> + Clone,
{
	type Service = SessionManager<Inner, Store, ReqBody, Auth>;

	fn layer(&self, inner: Inner) -> Self::Service
	{
		SessionManager::new(
			self.strict,
			Arc::clone(&self.cookie_options),
			self.store.clone(),
			inner,
		)
		.with_authorization(self.authorization.clone())
	}
}
