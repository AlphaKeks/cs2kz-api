//! This module contains the HTTP handlers for the `/servers` endpoint.

#![allow(unused_imports, dead_code)]

use std::io;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{self, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{routing, Router};
use futures::{SinkExt, TryStreamExt};
use problem_details::AsProblemDetails;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tower::ServiceBuilder;
use tower_sessions::{AuthorizeSession, CookieOptions, SessionManagerLayer, Strict};

use super::{
	get_server,
	get_servers,
	register_server,
	reset_access_key,
	update_server,
	websocket,
	AccessKey,
	ServerID,
	ServerIdentifier,
	ServerService,
};
use crate::auth::authorization::{HasPermissions, IsServerOwner};
use crate::auth::{Permissions, Session, SessionStore};
use crate::extract::{Extension, Json, Path, Query};
use crate::http::{Created, NoContent, Problem};
use crate::middleware::infallible::InfallibleLayer;
use crate::services::{MapService, PlayerService};

mod auth;
use auth::auth_websocket;

#[derive(Debug, Clone)]
struct WebSocketState
{
	heartbeat_interval: Duration,
	shutdown_token: CancellationToken,
	task_tracker: TaskTracker,
}

/// Returns a router for the `/servers` endpoint.
pub fn router(
	server_service: ServerService,
	cookie_options: Arc<CookieOptions>,
	store: SessionStore,
	websocket_heartbeat_interval: Duration,
	shutdown_token: CancellationToken,
) -> (TaskTracker, Router)
{
	let task_tracker = TaskTracker::new();

	let is_admin = HasPermissions(Permissions::MANAGE_SERVERS);
	let is_server_owner = IsServerOwner::new(server_service.mysql.clone());
	let session_auth =
		SessionManagerLayer::new(Strict::RequireAuthorization, cookie_options, store);

	let admin_only = ServiceBuilder::new()
		.layer(InfallibleLayer::new())
		.map_err(crate::auth::error::SessionManagerError::from)
		.layer(session_auth.clone().with_authorization(is_admin));

	let admin_or_owner = ServiceBuilder::new()
		.layer(InfallibleLayer::new())
		.map_err(crate::auth::error::SessionManagerError::from)
		.layer(session_auth.with_authorization(is_admin.or(is_server_owner)));

	let router = Router::new()
		.route("/", routing::get(get_servers))
		.route(
			"/",
			routing::post(register_server).route_layer(admin_only.clone()),
		)
		.route("/:server", routing::get(get_server))
		.route(
			"/:server",
			routing::patch(update_server).route_layer(admin_only.clone()),
		)
		.route(
			"/:server/access_key",
			routing::put(reset_access_key).route_layer(admin_or_owner),
		)
		.route(
			"/:server/access_key",
			routing::delete(clear_access_key).route_layer(admin_only),
		)
		.with_state(server_service.clone());

	let websocket_auth =
		axum::middleware::from_fn_with_state(server_service.mysql.clone(), auth_websocket);

	let websocket_router = Router::new()
		.route(
			"/websocket",
			routing::get(websocket).route_layer(websocket_auth),
		)
		.with_state(WebSocketState {
			heartbeat_interval: websocket_heartbeat_interval,
			shutdown_token,
			task_tracker: task_tracker.clone(),
		});

	(task_tracker, router.merge(websocket_router))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_server(
	State(server_service): State<ServerService>,
	Path(server_identifier): Path<ServerIdentifier>,
) -> crate::http::Result<Json<get_server::Response>>
{
	let server = match server_identifier {
		ServerIdentifier::ID(server_id) => server_service.get_server_by_id(server_id).await,
		ServerIdentifier::Name(server_name) => {
			server_service.get_server_by_name(&server_name).await
		}
	}
	.map_err(|error| error.as_problem_details())?;

	Ok(Json(server))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn get_servers(
	State(server_service): State<ServerService>,
	Query(request): Query<get_servers::Request>,
) -> crate::http::Result<Json<get_servers::Response>>
{
	let response = server_service
		.get_servers(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(response))
}

#[instrument(level = "debug")]
async fn websocket(
	State(WebSocketState {
		heartbeat_interval,
		shutdown_token,
		task_tracker,
	}): State<WebSocketState>,
	Extension(access_key): Extension<AccessKey>,
	Extension(server_id): Extension<ServerID>,
	upgrade: WebSocketUpgrade,
) -> crate::http::Response
{
	if task_tracker.is_closed() {
		return crate::http::problem::GracefulShutdown
			.as_problem_details()
			.into_response();
	}

	upgrade.on_upgrade(move |socket| async move {
		/// If the error is already an I/O error we downcast it to avoid extra
		/// indirections.
		fn axum_to_io_err(error: axum::Error) -> io::Error
		{
			let error = error.into_inner();

			error
				.downcast::<io::Error>()
				.map_or_else(io::Error::other, |boxed| *boxed)
		}

		let mut socket = socket.map_err(axum_to_io_err).sink_map_err(axum_to_io_err);
		let connection = match websocket::Connection::establish(&mut socket).await {
			Ok(conn) => conn,
			Err(error) => {
				if let Err(error) = socket
					.send(ws::Message::Close(Some(
						websocket::connection::CloseReason::ClientError {
							message: error.to_string().into(),
						}
						.as_close_frame(),
					)))
					.await
				{
					warn!(?error, "failed to close websocket connection");
				}

				return;
			}
		};

		if let Err(error) = task_tracker
			.track_future(websocket::serve_connection(
				connection,
				heartbeat_interval,
				shutdown_token,
			))
			.await
		{
			if let Err(error) = socket
				.send(ws::Message::Close(Some(
					websocket::connection::CloseReason::ClientError {
						message: error.to_string().into(),
					}
					.as_close_frame(),
				)))
				.await
			{
				warn!(?error, "failed to close websocket connection");
			}
		}
	})
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
#[axum::debug_handler]
async fn register_server(
	State(server_service): State<ServerService>,
	Extension(session): Extension<Session>,
	Json(request): Json<register_server::Request>,
) -> crate::http::Result<Created<register_server::Response>>
{
	sanity_check!(session
		.data()
		.permissions()
		.contains(Permissions::MANAGE_SERVERS));

	let response = server_service
		.register_server(request)
		.await
		.map_err(|error| error.as_problem_details())?;

	let location = location!("/servers/{}", response.server_id);

	Ok(Created::new(location, response))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn update_server(
	State(server_service): State<ServerService>,
	Path(server_id): Path<ServerID>,
	Json(request): Json<update_server::Request>,
) -> crate::http::Result<NoContent>
{
	server_service
		.update_server(server_id, request)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(NoContent)
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn reset_access_key(
	State(server_service): State<ServerService>,
	Extension(session): Extension<Session>,
	access_key: Option<Extension<AccessKey>>,
	Path(server_id): Path<ServerID>,
) -> crate::http::Result<Json<reset_access_key::Response>>
{
	#[derive(Debug, Error)]
	#[error("you cannot re-active your own server unless you are an admin")]
	struct ServerOwnerWithInvalidKeyRejection;

	impl AsProblemDetails for ServerOwnerWithInvalidKeyRejection
	{
		type ProblemType = Problem;

		fn problem_type(&self) -> Self::ProblemType
		{
			Problem::Unauthorized
		}
	}

	let is_admin = session
		.data()
		.permissions()
		.contains(Permissions::MANAGE_SERVERS);

	match (is_admin, access_key) {
		// Because the user isn't an admin, the only way they could have been authorized is
		// because they're the server owner. If their current key is invalid though,
		// they're not allowed to reset it.
		(false, Some(Extension(key))) if !key.is_valid() => {
			return Err(ServerOwnerWithInvalidKeyRejection.as_problem_details());
		}

		// If the user is the server owner, that must mean we inserted their access key
		// during authorization.
		(false, None) => unreachable!(),

		// If the user is an admin however, we don't even fetch the access key.
		(true, Some(_)) => unreachable!(),

		// The user is the server owner and their key is still valid -> fine
		(false, Some(_)) => {}

		// The user is an admin -> they can do whatever they want
		(true, None) => {
			sanity_check!(session
				.data()
				.permissions()
				.contains(Permissions::MANAGE_SERVERS));
		}
	}

	let access_key = server_service
		.reset_access_key(server_id)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(Json(access_key))
}

#[instrument(level = "debug", err(Debug, level = "debug"))]
async fn clear_access_key(
	State(server_service): State<ServerService>,
	Extension(session): Extension<Session>,
	Path(server_id): Path<ServerID>,
) -> crate::http::Result<NoContent>
{
	sanity_check!(session
		.data()
		.permissions()
		.contains(Permissions::MANAGE_SERVERS));

	server_service
		.clear_access_key(server_id)
		.await
		.map_err(|error| error.as_problem_details())?;

	Ok(NoContent)
}
