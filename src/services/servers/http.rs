//! HTTP handlers for this service.

use std::time::Duration;

use axum::extract::{State, WebSocketUpgrade};
use axum::{routing, Extension, Router};
use axum_extra::headers::authorization::Bearer;
use axum_extra::headers::Authorization;
use axum_extra::TypedHeader;
use cs2kz::SteamID;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tokio_util::task::task_tracker::TaskTracker;
use tower::ServiceBuilder;

use super::{
	websocket,
	ApiKey,
	DeleteKeyRequest,
	DeleteKeyResponse,
	Error,
	FetchServerRequest,
	FetchServerResponse,
	FetchServersRequest,
	FetchServersResponse,
	GenerateAccessTokenRequest,
	GenerateAccessTokenResponse,
	Host,
	RegisterServerRequest,
	RegisterServerResponse,
	ResetKeyRequest,
	ResetKeyResponse,
	ServerService,
	UpdateServerRequest,
	UpdateServerResponse,
};
use crate::http::extract::{Json, Path, Query};
use crate::http::ProblemDetails;
use crate::middleware;
use crate::services::auth::session::user::Permissions;
use crate::services::auth::session::{authorization, SessionManagerLayer};
use crate::services::auth::Session;
use crate::services::servers::ServerID;
use crate::util::ServerIdentifier;

/// State for the websocket endpoint.
#[derive(Debug, Clone)]
struct WebSocketState
{
	/// The interval at which clients should send heartbeat messages.
	heartbeat_interval: Duration,

	/// Used for graceful shutdown.
	cancellation_token: CancellationToken,

	/// Used for graceful shutdown.
	task_tracker: TaskTracker,
}

impl From<ServerService> for Router
{
	fn from(svc: ServerService) -> Self
	{
		let admin_only = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::RequiredPermissions(Permissions::SERVERS),
			));

		let owner_auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer::new())
			.layer(SessionManagerLayer::with_strategy(
				svc.auth_svc.clone(),
				authorization::IsServerOwner::new(svc.database.clone()),
			));

		let no_cors = Router::new()
			// TODO: remove
			.route("/auth", routing::post(generate_access_token))
			.route(
				"/ws",
				routing::get(websocket).layer(Extension(WebSocketState {
					heartbeat_interval: Duration::from_secs(10),
					cancellation_token: svc.websocket_cancellation_token.child_token(),
					task_tracker: svc.websocket_task_tracker.clone(),
				})),
			)
			.with_state(svc.clone());

		let public = Router::new()
			.route("/", routing::get(get_many))
			.route("/:server", routing::get(get_single))
			.route_layer(middleware::cors::permissive())
			.with_state(svc.clone());

		let protected = Router::new()
			.route("/", routing::post(register_server).route_layer(admin_only.clone()))
			.route("/:server", routing::patch(update_server).route_layer(owner_auth.clone()))
			.route("/:server/key", routing::put(reset_api_key).route_layer(owner_auth.clone()))
			.route("/:server/key", routing::delete(delete_api_key).route_layer(admin_only.clone()))
			.route_layer(middleware::cors::dashboard([
				http::Method::OPTIONS,
				http::Method::POST,
				http::Method::PATCH,
				http::Method::PUT,
				http::Method::DELETE,
			]))
			.with_state(svc.clone());

		no_cors.merge(public).merge(protected)
	}
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/servers", tag = "Servers", params(FetchServersRequest))]
async fn get_many(
	State(svc): State<ServerService>,
	Query(req): Query<FetchServersRequest>,
) -> Result<FetchServersResponse, ProblemDetails>
{
	let res = svc.fetch_servers(req).await?;

	if res.servers.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/servers", tag = "Servers", security(("Browser serssion" = ["servers"])))]
async fn register_server(
	session: Session,
	State(svc): State<ServerService>,
	Json(req): Json<RegisterServerRequest>,
) -> Result<RegisterServerResponse, ProblemDetails>
{
	let res = svc.register_server(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(post, path = "/servers/auth", tag = "Servers")]
async fn generate_access_token(
	State(svc): State<ServerService>,
	Json(req): Json<GenerateAccessTokenRequest>,
) -> Result<GenerateAccessTokenResponse, ProblemDetails>
{
	let res = svc.generate_access_token(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
async fn websocket(
	State(svc): State<ServerService>,
	Extension(WebSocketState { heartbeat_interval, cancellation_token, task_tracker }): Extension<
		WebSocketState,
	>,
	TypedHeader(auth_header): TypedHeader<Authorization<Bearer>>,
	upgrade: WebSocketUpgrade,
) -> Result<axum::response::Response, ProblemDetails>
{
	#[expect(clippy::missing_docs_in_private_items)]
	#[derive(Debug, thiserror::Error)]
	#[error("you are not authorized to perform this action")]
	struct Unauthorized;

	impl crate::http::problem_details::IntoProblemDetails for Unauthorized
	{
		fn problem_type(&self) -> crate::http::problem_details::ProblemType
		{
			crate::http::problem_details::ProblemType::Unauthorized
		}
	}

	let key = auth_header.token().parse::<ApiKey>()?;
	let server_id = sqlx::query_scalar! {
		"SELECT id `id: ServerID`
		 FROM Servers
		 WHERE `key` = ?",
		key,
	}
	.fetch_optional(&svc.database)
	.await?
	.ok_or(Unauthorized)?;

	tracing::info!(id = %server_id, "authenticated server");

	Ok(upgrade.on_upgrade(move |mut socket| async move {
		let mut conn = match websocket::Connection::establish(
			&mut socket,
			heartbeat_interval,
			cancellation_token,
			server_id,
			svc.map_svc,
			svc.player_svc,
		)
		.await
		{
			Ok(conn) => conn,
			Err(websocket::connection::EstablishConnectionError::Handshake(
				websocket::connection::HandshakeError::Timeout,
			)) => {
				tracing::warn!("handshake timed out");
				return;
			}
			Err(websocket::connection::EstablishConnectionError::Handshake(
				websocket::connection::HandshakeError::ConnectionClosed { close_frame },
			)) => {
				tracing::warn!(?close_frame, "connection closed immediately");
				return;
			}
			Err(websocket::connection::EstablishConnectionError::Handshake(
				websocket::connection::HandshakeError::EncodeAck(error),
			)) => {
				tracing::error!(?error, "failed to encode HelloACK");
				return;
			}
			Err(websocket::connection::EstablishConnectionError::Handshake(
				websocket::connection::HandshakeError::Io(error),
			)) => {
				tracing::warn!(?error, "something went wrong");
				return;
			}
		};

		match task_tracker.track_future(conn.serve()).await {
			Ok(()) => {
				tracing::trace!(%server_id, "finished ws conn");
			}
			Err(websocket::connection::ServeConnectionError::SendMessage(error)) => {
				tracing::error!(?error, "failed to send message");
				conn.close(websocket::CloseReason::Error(error.into()))
					.await;
			}
			Err(_) => unreachable!("we handle the other errors gracefully"),
		}
	}))
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(get, path = "/servers/{server}", tag = "Servers", params(
  ("server" = ServerIdentifier, Path, description = "a server's ID or name"),
))]
async fn get_single(
	State(svc): State<ServerService>,
	Path(identifier): Path<ServerIdentifier>,
) -> Result<FetchServerResponse, ProblemDetails>
{
	let req = FetchServerRequest { identifier };
	let res = svc
		.fetch_server(req)
		.await?
		.ok_or(Error::ServerDoesNotExist)?;

	Ok(res)
}

/// Request payload for `PATCH /servers/{server}`
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[schema(title = "UpdateServerRequest")]
#[doc(hidden)]
pub(crate) struct UpdateServerRequestPayload
{
	/// A new name.
	pub new_name: Option<String>,

	/// A new host.
	pub new_host: Option<Host>,

	/// A new port.
	pub new_port: Option<u16>,

	/// SteamID of a new owner.
	pub new_owner: Option<SteamID>,
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  patch,
  path = "/servers/{server_id}",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn update_server(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
	Json(UpdateServerRequestPayload { new_name, new_host, new_port, new_owner }): Json<
		UpdateServerRequestPayload,
	>,
) -> Result<UpdateServerResponse, ProblemDetails>
{
	let req = UpdateServerRequest { server_id, new_name, new_host, new_port, new_owner };
	let res = svc.update_server(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  put,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn reset_api_key(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<ResetKeyResponse, ProblemDetails>
{
	let req = ResetKeyRequest { server_id };
	let res = svc.reset_key(req).await?;

	Ok(res)
}

#[tracing::instrument(err(Debug, level = "debug"))]
#[utoipa::path(
  delete,
  path = "/servers/{server_id}/key",
  tag = "Servers",
  params(("server_id" = ServerID, Path, description = "a server's ID")),
  security(("Browser Session" = ["servers"])),
)]
async fn delete_api_key(
	session: Session,
	State(svc): State<ServerService>,
	Path(server_id): Path<ServerID>,
) -> Result<DeleteKeyResponse, ProblemDetails>
{
	let req = DeleteKeyRequest { server_id };
	let res = svc.delete_key(req).await?;

	Ok(res)
}
