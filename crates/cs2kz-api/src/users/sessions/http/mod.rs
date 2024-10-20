use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::{routing, Router};
use futures::stream::TryStreamExt;

use self::middleware::{session_auth, SessionAuthState};
use crate::config::CookieConfig;
use crate::database;
use crate::http::extract::Json;
use crate::http::responses::ErrorResponse;
use crate::time::Timestamp;
use crate::users::sessions::{self, Session, SessionID};

pub(in crate::users) mod middleware;

/// Returns a router for `/users/current/sessions`.
pub fn router<S>(
	pool: database::ConnectionPool,
	cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let middleware = axum::middleware::from_fn_with_state(
		SessionAuthState::new(pool, cookie_config),
		session_auth,
	);

	Router::new()
		.route("/", routing::get(get_sessions).delete(invalidate_sessions))
		.route(
			"/current",
			routing::get(get_current_session).delete(invalidate_current_session),
		)
		.route_layer(middleware)
}

/// Response payload for a [session].
///
/// [session]: crate::users::sessions
#[derive(Debug, serde::Serialize)]
struct SessionInformation {
	id: SessionID,
	created_at: Timestamp,
	expires_at: Timestamp,
}

impl From<sessions::database::Session> for SessionInformation {
	fn from(session: sessions::database::Session) -> Self {
		Self {
			id: session.id,
			created_at: session.created_at,
			expires_at: session.expires_at,
		}
	}
}

/// Returns information about all active sessions for the current user.
#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
async fn get_sessions(
	State(pool): State<database::ConnectionPool>,
	session: Session,
) -> Result<Json<Vec<SessionInformation>>, ErrorResponse> {
	let mut conn = pool.get_connection().await?;
	let sessions = sessions::database::find_by_user(&mut conn, session.user().id())
		.map_ok(SessionInformation::from)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(Json(sessions))
}

/// Returns information about the currently active session.
#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
async fn get_current_session(
	State(pool): State<database::ConnectionPool>,
	session: Session,
) -> Result<Json<SessionInformation>, ErrorResponse> {
	let mut conn = pool.get_connection().await?;
	let session = sessions::database::find(&mut conn, session.id())
		.await?
		.map(SessionInformation::from)
		.ok_or(crate::http::responses::NotFound)?;

	Ok(Json(session))
}

/// Invalidates all active sessions for the current user.
#[instrument(skip(pool), err(level = "debug"))]
async fn invalidate_sessions(
	State(pool): State<database::ConnectionPool>,
	session: Session,
) -> Result<crate::http::responses::NoContent, ErrorResponse> {
	let mut txn = pool.begin_transaction().await?;

	sessions::database::invalidate_all(&mut txn, session.user().id()).await?;
	session.invalidate();

	Ok(crate::http::responses::NoContent)
}

/// Invalidates the currently active session.
#[instrument]
async fn invalidate_current_session(session: Session) -> crate::http::responses::NoContent {
	session.invalidate();
	crate::http::responses::NoContent
}
