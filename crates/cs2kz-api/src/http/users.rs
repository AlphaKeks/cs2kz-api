use std::future;
use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::routing::{self, MethodRouter};
use futures_util::TryStreamExt;

use crate::config::CookieConfig;
use crate::database;
use crate::http::extract::{Json, Path, Query};
use crate::http::middleware::session_auth::{SessionAuthState, session_auth};
use crate::http::response::{ErrorResponse, NoContent, NotFound};
use crate::users::sessions::{self, Session, SessionInfo};
use crate::users::{self, Permissions, User, UserID};

pub fn router<S>(
	database: database::ConnectionPool,
	cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let session_auth_state = SessionAuthState::new(database, cookie_config);
	let is_logged_in = axum::middleware::from_fn_with_state(session_auth_state, session_auth);

	Router::new()
		.route("/", routing::get(get_users))
		.route(
			"/current",
			routing::get(get_current_user).layer(is_logged_in.clone()),
		)
		.route("/{user_id}", routing::get(get_user_by_id))
		.route(
			"/current/sessions",
			MethodRouter::new()
				.get(get_sessions)
				.delete(delete_sessions)
				.layer(is_logged_in.clone()),
		)
		.route(
			"/current/sessions/current",
			MethodRouter::new()
				.get(get_current_session)
				.delete(delete_current_session)
				.layer(is_logged_in),
		)
}

#[derive(serde::Deserialize, utoipa::IntoParams)]
struct GetUsersQuery {
	/// Only include users with these specific permissions.
	#[serde(default)]
	permissions: Permissions,
}

/// Returns all users with permissions.
#[instrument]
#[utoipa::path(get, path = "/users", tag = "Users", params(GetUsersQuery), responses(
	(status = 200, body = [User]),
))]
pub(crate) async fn get_users(
	mut db_conn: database::Connection,
	Query(GetUsersQuery { permissions }): Query<GetUsersQuery>,
) -> Result<Json<Vec<User>>, ErrorResponse> {
	let users = users::get_with_permissions(&mut db_conn)
		.try_filter(|user| future::ready(user.permissions.contains(permissions)))
		.try_collect()
		.await?;

	Ok(Json(users))
}

/// Returns the currently logged-in user.
#[instrument]
#[utoipa::path(get, path = "/users/current", tag = "Users", responses(
	(status = 200, body = User),
))]
pub(crate) async fn get_current_user(session: Session) -> Json<User> {
	let user = session.user();

	Json(User {
		id: user.id(),
		permissions: user.permissions(),
	})
}

/// Returns a specific user.
#[instrument]
#[utoipa::path(get, path = "/users/{user_id}", tag = "Users", params(("user_id",)), responses(
	(status = 200, body = User),
))]
pub(crate) async fn get_user_by_id(
	mut db_conn: database::Connection,
	Path(user_id): Path<UserID>,
) -> Result<Json<User>, ErrorResponse> {
	let user = users::get_by_id(&mut db_conn, user_id)
		.await?
		.ok_or(NotFound)?;

	Ok(Json(user))
}

/// Returns all active sessions of the currently logged-in user.
#[instrument]
#[utoipa::path(get, path = "/users/current/sessions", tag = "Users", responses(
	(status = 200, body = [SessionInfo]),
))]
pub(crate) async fn get_sessions(
	session: Session,
	mut db_conn: database::Connection,
) -> Result<Json<Vec<SessionInfo>>, ErrorResponse> {
	let sessions = sessions::get_by_user(&mut db_conn, session.user().id())
		.try_collect()
		.await?;

	Ok(Json(sessions))
}

/// Expires all active sessions of the currently logged-in user.
#[instrument]
#[utoipa::path(delete, path = "/users/current/sessions", tag = "Users", responses(
	(status = 204),
))]
pub(crate) async fn delete_sessions(
	session: Session,
	mut db_conn: database::Connection,
) -> Result<NoContent, ErrorResponse> {
	sessions::invalidate_for_user(&mut db_conn, session.user().id()).await?;
	session.invalidate();

	Ok(NoContent)
}

/// Returns the session of the currently logged-in user.
#[instrument]
#[utoipa::path(get, path = "/users/current/sessions/current", tag = "Users", responses(
	(status = 200, body = SessionInfo),
))]
pub(crate) async fn get_current_session(
	session: Session,
	mut db_conn: database::Connection,
) -> Result<Json<SessionInfo>, ErrorResponse> {
	let session_info = sessions::get_by_id(&mut db_conn, session.id())
		.await?
		.expect("session is currently active, so it must be available");

	Ok(Json(session_info))
}

/// Expires the current session.
#[instrument]
#[utoipa::path(delete, path = "/users/current/sessions/current", tag = "Users", responses(
	(status = 204),
))]
pub(crate) async fn delete_current_session(session: Session) -> NoContent {
	session.invalidate();
	NoContent
}
