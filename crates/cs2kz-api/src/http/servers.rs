use std::sync::Arc;

use axum::Router;
use axum::extract::FromRef;
use axum::handler::Handler;
use axum::routing::MethodRouter;

use crate::config::CookieConfig;
use crate::database;
use crate::http::middleware::session_auth::{SessionAuthState, session_auth};
use crate::users::Permission;
use crate::users::sessions::authorization::{AuthorizeSession, HasPermissions, IsServerOwner};

pub fn router<S>(
	database: database::ConnectionPool,
	cookie_config: impl Into<Arc<CookieConfig>>,
) -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	let session_auth_state = SessionAuthState::new(database.clone(), cookie_config);

	let is_admin = axum::middleware::from_fn_with_state(
		session_auth_state
			.clone()
			.with_authorization(HasPermissions::new(Permission::Servers)),
		session_auth,
	);

	let is_admin_or_owner = axum::middleware::from_fn_with_state(
		session_auth_state.with_authorization(
			HasPermissions::new(Permission::Servers).or(IsServerOwner::new(database)),
		),
		session_auth,
	);

	Router::new()
		.route(
			"/",
			MethodRouter::new()
				.get(get_servers)
				.post(approve_server.layer(is_admin.clone())),
		)
		.route(
			"/{server}",
			MethodRouter::new()
				.get(get_server)
				.patch(update_server.layer(is_admin_or_owner.clone())),
		)
		.route(
			"/{server}/access-key",
			MethodRouter::new()
				.put(reset_server_access_key.layer(is_admin_or_owner))
				.delete(revoke_server_access_key.layer(is_admin)),
		)
}

async fn get_servers() {}
async fn approve_server() {}
async fn get_server() {}
async fn update_server() {}
async fn reset_server_access_key() {}
async fn revoke_server_access_key() {}
