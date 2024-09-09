use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use cookie::Cookie;
use tower_service::Service;

use crate::{
	authorization,
	AuthorizeSession,
	CookieOptions,
	Session,
	SessionID,
	SessionManagerError,
	SessionStore,
	Strict,
};

/// A middleware for performing session authentication & authorization on HTTP requests.
pub struct SessionManager<Inner, Store, ReqBody, Auth = authorization::None<ReqBody, Store>>
where
	Inner: Service<http::Request<ReqBody>>,
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

	/// The inner service.
	inner: Inner,
}

impl<Inner, Store, ReqBody> SessionManager<Inner, Store, ReqBody>
where
	Inner: Service<http::Request<ReqBody>>,
	Store: SessionStore,
	ReqBody: Send + 'static,
{
	/// Constructs a new [`SessionManager`].
	pub fn new(
		strict: Strict,
		cookie_options: impl Into<Arc<CookieOptions>>,
		store: Store,
		inner: Inner,
	) -> Self
	{
		Self {
			strict,
			cookie_options: cookie_options.into(),
			authorization: authorization::None::new(),
			store,
			inner,
		}
	}
}

impl<Inner, Store, ReqBody, Auth> SessionManager<Inner, Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>>,
	Store: SessionStore,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
{
	/// Sets the authorization strategy.
	pub fn with_authorization<NewAuth>(
		self,
		authorization: NewAuth,
	) -> SessionManager<Inner, Store, ReqBody, NewAuth>
	where
		NewAuth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	{
		SessionManager {
			strict: self.strict,
			cookie_options: self.cookie_options,
			authorization,
			store: self.store,
			inner: self.inner,
		}
	}

	/// Get a reference to the inner service.
	pub fn get_ref(&self) -> &Inner
	{
		&self.inner
	}

	/// Get a mutable reference to the inner service.
	pub fn get_mut(&mut self) -> &mut Inner
	{
		&mut self.inner
	}

	/// Consume the middleware and return the inner service.
	pub fn into_inner(self) -> Inner
	{
		self.inner
	}
}

impl<Inner, Store, Auth, ReqBody, ResBody> SessionManager<Inner, Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>, Response = http::Response<ResBody>> + Send,
	Inner::Error: std::error::Error + 'static,
	Inner::Future: Send,
	Store: SessionStore,
	Store::ID: Clone,
	Store::Data: Clone,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	ReqBody: Send,
	ResBody: Send,
{
	async fn call_impl(
		mut self,
		mut request: http::Request<ReqBody>,
	) -> Result<http::Response<ResBody>, SessionManagerError<Store, Auth, ReqBody, Inner>>
	{
		let session_id = extract_session_id(request.headers())?;
		let maybe_session = self
			.store
			.load_session(&session_id)
			.await
			.map(|data| Session::new(session_id, data));

		let session = match (self.strict, maybe_session) {
			(Strict::Lax, maybe_session) => {
				let maybe_session = maybe_session.ok();
				request.extensions_mut().insert(maybe_session.clone());
				maybe_session
			}
			(Strict::RequireAuthentication, Ok(mut session)) => {
				session.authenticate();

				if self
					.authorization
					.authorize_session(&session, &mut request)
					.await
					.is_ok()
				{
					session.authorize();
				}

				request.extensions_mut().insert(session.clone());
				Some(session)
			}
			(Strict::RequireAuthorization, Ok(mut session)) => {
				self.authorization
					.authorize_session(&session, &mut request)
					.await
					.map_err(SessionManagerError::AuthorizeSession)?;

				session.authorize();
				request.extensions_mut().insert(session.clone());
				Some(session)
			}
			(_, Err(error)) => {
				return Err(SessionManagerError::AuthenticateSession(error));
			}
		};

		let mut response = self
			.inner
			.call(request)
			.await
			.map_err(SessionManagerError::Service)?;

		if let Some(session) = session {
			let mut cookie = make_cookie::<Store>(&session, &self.cookie_options);

			if session.is_valid() {
				let expiration = self
					.store
					.save_session(session)
					.await
					.map_err(SessionManagerError::SaveSession)?;

				cookie.set_expires(expiration);
			} else {
				self.store
					.invalidate_session(session)
					.await
					.map_err(SessionManagerError::InvalidateSession)?;

				cookie.make_removal();
			}

			let cookie = cookie
				.encoded()
				.to_string()
				.parse::<http::HeaderValue>()
				.expect("cookie should be valid http header value");

			response
				.headers_mut()
				.append(http::header::SET_COOKIE, cookie);
		}

		Ok(response)
	}
}

fn extract_session_id<Store, Auth, ReqBody, Inner>(
	headers: &http::HeaderMap,
) -> Result<Store::ID, SessionManagerError<Store, Auth, ReqBody, Inner>>
where
	Store: SessionStore,
	<Store::ID as SessionID>::Error: std::error::Error,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store>,
	Inner: Service<http::Request<ReqBody>>,
	Inner::Error: std::error::Error,
{
	let cookie_name = <Store::ID as SessionID>::cookie_name();
	headers
		.get_all(http::header::COOKIE)
		.into_iter()
		.flat_map(|v| v.to_str())
		.flat_map(|v| Cookie::split_parse_encoded(v.trim().to_owned()))
		.flatten()
		.find(|cookie| cookie.name() == cookie_name)
		.map(|cookie| <Store::ID as SessionID>::decode(cookie.value()))
		.ok_or(SessionManagerError::MissingSessionID)?
		.map_err(SessionManagerError::DecodeSessionID)
}

fn make_cookie<Store>(session: &Session<Store>, options: &CookieOptions) -> Cookie<'static>
where
	Store: SessionStore,
{
	Cookie::build((
		<Store::ID as SessionID>::cookie_name(),
		<Store::ID as SessionID>::encode(session.id()),
	))
	.domain(options.domain.clone())
	.path(options.path.clone())
	.secure(options.secure)
	.http_only(options.http_only)
	.same_site(options.same_site)
	.build()
}

impl<Inner, Store, Auth, ReqBody, ResBody> Service<http::Request<ReqBody>>
	for SessionManager<Inner, Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>, Response = http::Response<ResBody>>
		+ Clone
		+ Send
		+ 'static,
	Inner::Error: std::error::Error,
	Inner::Future: Send,
	Store: SessionStore + Clone,
	Store::ID: Clone,
	Store::Data: Clone,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store> + Clone,
	ReqBody: Send + 'static,
	ResBody: Send + 'static,
{
	type Response = http::Response<ResBody>;
	type Error = SessionManagerError<Store, Auth, ReqBody, Inner>;
	type Future =
		Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

	fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>
	{
		self.inner.poll_ready(cx).map_err(<Self::Error>::Service)
	}

	fn call(&mut self, request: http::Request<ReqBody>) -> Self::Future
	{
		Box::pin(self.clone().call_impl(request))
	}
}

impl<Inner, Store, Auth, ReqBody> fmt::Debug for SessionManager<Inner, Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>> + fmt::Debug,
	Store: SessionStore + fmt::Debug,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store> + fmt::Debug,
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("SessionManager")
			.field("strict", &self.strict)
			.field("cookie_options", &*self.cookie_options)
			.field("authorization", &self.authorization)
			.field("store", &self.store)
			.field("inner", &self.inner)
			.finish()
	}
}

impl<Inner, Store, Auth, ReqBody> Clone for SessionManager<Inner, Store, ReqBody, Auth>
where
	Inner: Service<http::Request<ReqBody>> + Clone,
	Store: SessionStore + Clone,
	Auth: AuthorizeSession<ReqBody = ReqBody, Store = Store> + Clone,
{
	fn clone(&self) -> Self
	{
		Self {
			strict: self.strict,
			cookie_options: Arc::clone(&self.cookie_options),
			authorization: self.authorization.clone(),
			store: self.store.clone(),
			inner: self.inner.clone(),
		}
	}

	fn clone_from(&mut self, source: &Self)
	{
		self.strict = source.strict;
		self.cookie_options.clone_from(&source.cookie_options);
		self.authorization.clone_from(&source.authorization);
		self.store.clone_from(&source.store);
		self.inner.clone_from(&source.inner);
	}
}
