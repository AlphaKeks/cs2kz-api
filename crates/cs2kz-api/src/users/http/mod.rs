use std::sync::Arc;

use axum::extract::{FromRef, State};
use axum::{routing, Router};

use crate::config::CookieConfig;
use crate::database::{self};
use crate::http::extract::{Json, Path, Query};
use crate::http::responses::ErrorResponse;
use crate::users::permissions::{Permission, Permissions};
use crate::users::sessions::authorization::RequiredPermissions;
use crate::users::sessions::http::middleware::{session_auth, SessionAuthState};
use crate::users::sessions::Session;
use crate::users::{self, GetUsersParams, User, UserID};

/// Returns a router for `/users`.
pub fn router<S>(
	pool: database::ConnectionPool,
	cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let cookie_config: Arc<CookieConfig> = cookie_config.into();
	let auth_state = SessionAuthState::new(pool.clone(), Arc::clone(&cookie_config));
	let is_logged_in = axum::middleware::from_fn_with_state(auth_state.clone(), session_auth);
	let is_admin = axum::middleware::from_fn_with_state(
		auth_state.with_authz(RequiredPermissions(Permission::Admin.into())),
		session_auth,
	);

	Router::new()
		.route("/", routing::get(get_users))
		.route("/current", routing::get(get_current_user).route_layer(is_logged_in))
		.route("/{user_id}", routing::get(get_user))
		.route("/{user_id}", routing::patch(update_user).route_layer(is_admin))
		.nest("/current/sessions", users::sessions::http::router(pool, cookie_config))
}

/// Returns all users with the permissions specified in the query parameters.
///
/// Users without any permissions will never be returned.
#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
#[utoipa::path(get, path = "/users", tag = "Users", params(GetUsersParams), responses(
	(status = 200, body = Vec<User>),
))]
async fn get_users(
	State(pool): State<database::ConnectionPool>,
	Query(params): Query<GetUsersParams>,
) -> Result<Json<Vec<User>>, ErrorResponse> {
	let users = users::get_many(&pool, params).await?;

	Ok(Json(users))
}

/// Returns information about the currently logged in user.
#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
#[utoipa::path(get, path = "/users/current", tag = "Users", security(("session" = [])), responses(
	(status = 200, body = User),
))]
async fn get_current_user(
	State(pool): State<database::ConnectionPool>,
	session: Session,
) -> Result<Json<User>, ErrorResponse> {
	let user = users::get(&pool, session.user().id())
		.await?
		.expect("if authentication was successful, this user should definitely exist");

	Ok(Json(user))
}

/// Returns information about the user with the given `user_id`.
#[instrument(skip(pool), ret(level = "debug"), err(level = "debug"))]
async fn get_user(
	State(pool): State<database::ConnectionPool>,
	Path(user_id): Path<UserID>,
) -> Result<Json<User>, ErrorResponse> {
	let user = users::get(&pool, user_id)
		.await?
		.ok_or(crate::http::responses::NotFound)?;

	Ok(Json(user))
}

#[derive(serde::Deserialize)]
struct UserUpdate {
	/// Overwrite the user's permissions.
	permissions: Option<Permissions>,
}

/// Updates the user of the given `user_id`.
#[instrument(skip(pool), err(level = "debug"))]
async fn update_user(
	State(pool): State<database::ConnectionPool>,
	session: Session,
	Path(user_id): Path<UserID>,
	Json(UserUpdate { permissions }): Json<UserUpdate>,
) -> Result<crate::http::responses::NoContent, ErrorResponse> {
	match users::update(&pool, users::UserUpdate {
		user_id,
		permissions,
		email: None,
		mark_as_seen: false,
	})
	.await
	{
		Ok(true) => Ok(crate::http::responses::NoContent),
		Ok(false) => Err(crate::http::responses::NotFound.into()),
		Err(error) => Err(error.into()),
	}
}
