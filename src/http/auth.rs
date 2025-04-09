use {
	axum::{
		extract::{FromRequestParts, OptionalFromRequestParts},
		response::{IntoResponse, Response},
	},
	cs2kz_api::users::{Permissions, ServerBudget, UserId, Username, sessions::SessionId},
	http::request,
	std::{
		convert::Infallible,
		sync::{
			Arc,
			atomic::{self, AtomicBool},
		},
	},
};

#[derive(Debug, Clone)]
pub(crate) struct Session
{
	id: SessionId,
	user_info: UserInfo,
	is_valid: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Builder)]
#[builder(builder_type(vis = "pub(super)"))]
pub(crate) struct UserInfo
{
	id: UserId,
	name: Username,
	permissions: Permissions,
	server_budget: ServerBudget,
}

impl Session
{
	pub(super) fn new(id: SessionId, user_info: UserInfo) -> Self
	{
		Self { id, user_info, is_valid: Arc::new(AtomicBool::new(true)) }
	}

	pub(crate) fn id(&self) -> SessionId
	{
		self.id
	}

	pub(crate) fn user_info(&self) -> &UserInfo
	{
		&self.user_info
	}

	pub(crate) fn is_valid(&self) -> bool
	{
		self.is_valid.load(atomic::Ordering::SeqCst)
	}

	/// Marks this session as invalid.
	///
	/// Returns whether the session was previously valid.
	pub(crate) fn invalidate(&self) -> bool
	{
		self.is_valid.swap(false, atomic::Ordering::SeqCst)
	}
}

#[derive(Debug, Display, Clone)]
#[display("no session found in extensions")]
pub(crate) struct SessionRejection(());

impl IntoResponse for SessionRejection
{
	fn into_response(self) -> Response
	{
		http::StatusCode::UNAUTHORIZED.into_response()
	}
}

impl<S> FromRequestParts<S> for Session
where
	S: Send + Sync,
{
	type Rejection = SessionRejection;

	#[instrument(level = "debug", skip_all, ret(level = "debug"), err(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection>
	{
		match <Self as OptionalFromRequestParts<S>>::from_request_parts(parts, state).await {
			Ok(Some(session)) => Ok(session),
			Ok(None) => Err(SessionRejection(())),
		}
	}
}

impl<S> OptionalFromRequestParts<S> for Session
where
	S: Send + Sync,
{
	type Rejection = Infallible;

	#[instrument(level = "debug", skip_all, ret(level = "debug"))]
	async fn from_request_parts(
		parts: &mut request::Parts,
		_state: &S,
	) -> Result<Option<Self>, Self::Rejection>
	{
		Ok(parts.extensions.get::<Self>().cloned())
	}
}

impl UserInfo
{
	pub(crate) fn id(&self) -> UserId
	{
		self.id
	}

	#[expect(dead_code, reason = "API consistency")]
	pub(crate) fn name(&self) -> &Username
	{
		&self.name
	}

	pub(crate) fn permissions(&self) -> Permissions
	{
		self.permissions
	}

	pub(crate) fn server_budget(&self) -> ServerBudget
	{
		self.server_budget
	}
}
