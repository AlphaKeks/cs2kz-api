use {
	crate::{Config, http::auth},
	axum::{
		extract::{Request, State},
		middleware::Next,
		response::{IntoResponse, Response},
	},
	axum_extra::extract::CookieJar,
	cs2kz_api::{
		database::{self, DatabaseError, DatabaseResult},
		time::Timestamp,
		users::{self, sessions::SessionId},
	},
	std::sync::Arc,
};

pub(crate) macro layer($database:expr, $config:expr) {
	axum::middleware::from_fn_with_state(
		AuthState { database: $database.into(), config: $config.into() },
		middleware_fn,
	)
}

#[derive(Debug, Clone)]
pub(crate) struct AuthState
{
	database: database::ConnectionPool,
	config: Arc<Config>,
}

#[derive(Debug, Display, Error, From)]
pub(crate) enum SessionRejection
{
	#[display("session ID is invalid")]
	InvalidSessionId,

	#[display("session is expired")]
	SessionExpired,

	#[display("database error: {_0}")]
	DatabaseError(DatabaseError),
}

impl IntoResponse for SessionRejection
{
	fn into_response(self) -> Response
	{
		match self {
			SessionRejection::InvalidSessionId | SessionRejection::SessionExpired => {
				http::StatusCode::UNAUTHORIZED.into_response()
			},
			SessionRejection::DatabaseError(_) => {
				http::StatusCode::INTERNAL_SERVER_ERROR.into_response()
			},
		}
	}
}

#[instrument(level = "debug", skip(database, config, req, next), err(Debug, level = "debug"))]
pub(crate) async fn middleware_fn(
	State(AuthState { database, config }): State<AuthState>,
	maybe_session_id: Option<SessionId>,
	mut req: Request,
	next: Next,
) -> Result<Response, SessionRejection>
{
	let Some(session_id) = maybe_session_id else {
		return Ok(next.run(req).await);
	};

	let session = {
		let mut db_conn = database.acquire().await?;
		users::sessions::get_by_id(&mut db_conn, session_id)
			.await?
			.ok_or(SessionRejection::InvalidSessionId)?
	};

	if session.expires_at <= Timestamp::now() {
		return Err(SessionRejection::SessionExpired);
	}

	let user_info = auth::UserInfo::builder()
		.id(session.user.id)
		.name(session.user.name)
		.permissions(session.user.permissions)
		.server_budget(session.user.server_budget)
		.build();

	let session = auth::Session::new(session.id, user_info);

	req.extensions_mut().insert(session.clone());

	let response = next.run(req).await;
	let mut cookie = config
		.http
		.cookies
		.auth_cookie_builder(SessionId::COOKIE_NAME, session.id().to_string())
		.build();

	database
		.in_transaction(async |db_conn| -> DatabaseResult<()> {
			let updated = if session.is_valid() {
				users::sessions::extend(session.id())
					.duration(config.http.session_duration)
					.exec(db_conn)
					.await?
			} else {
				cookie.make_removal();
				users::sessions::expire(session.id()).exec(db_conn).await?
			};

			if !updated {
				warn!(session.id = %session.id(), "updated non-existent session?");
			}

			Ok(())
		})
		.await?;

	Ok((CookieJar::default().add(cookie), response).into_response())
}
