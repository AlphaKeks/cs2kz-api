//! HTTP handlers for this service.

use axum::extract::{Path, State};
use axum::{routing, Json, Router};
use axum_extra::extract::Query;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use tower::ServiceBuilder;

use super::models::UnbanReason;
use super::{
	BanID,
	BanRequest,
	BanResponse,
	BanService,
	Error,
	FetchBanRequest,
	FetchBanResponse,
	FetchBansRequest,
	FetchBansResponse,
	UnbanRequest,
	UnbanResponse,
	UpdateBanRequest,
};
use crate::http::response::NoContent;
use crate::middleware;
use crate::runtime::Result;
use crate::services::auth::session::authorization::RequiredPermissions;
use crate::services::auth::session::{user, SessionManagerLayer};
use crate::services::auth::Session;

impl From<BanService> for Router
{
	fn from(svc: BanService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer)
			.layer(SessionManagerLayer::with_authorization(
				svc.auth_svc.clone(),
				RequiredPermissions(user::Permissions::BANS),
			));

		Router::new()
			.route("/", routing::get(get_many))
			.route("/", routing::post(create).route_layer(auth.clone()))
			.route("/:id", routing::get(get_single))
			.route("/:id", routing::patch(update).route_layer(auth.clone()))
			.route("/:id", routing::delete(revert).route_layer(auth.clone()))
			.with_state(svc)
	}
}

/// Fetch a specific ban by its ID.
async fn get_single(
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
) -> Result<Json<FetchBanResponse>>
{
	let res = svc
		.fetch_ban(FetchBanRequest { ban_id })
		.await?
		.ok_or(Error::BanDoesNotExist { ban_id })?;

	Ok(Json(res))
}

/// Fetch many bans.
async fn get_many(
	State(svc): State<BanService>,
	Query(req): Query<FetchBansRequest>,
) -> Result<Json<FetchBansResponse>>
{
	let res = svc.fetch_bans(req).await?;

	if res.bans.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(Json(res))
}

/// Ban a player.
async fn create(
	State(svc): State<BanService>,
	Json(req): Json<BanRequest>,
) -> Result<Json<BanResponse>>
{
	let res = svc.ban_player(req).await?;

	Ok(Json(res))
}

#[derive(Debug, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct UpdateBanRequestPayload
{
	new_reason: Option<String>,
	new_expiration_date: Option<DateTime<Utc>>,
}

/// Update a ban.
async fn update(
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(req): Json<UpdateBanRequestPayload>,
) -> Result<NoContent>
{
	let req = UpdateBanRequest {
		ban_id,
		new_reason: req.new_reason,
		new_expiration_date: req.new_expiration_date,
	};

	svc.update_ban(req).await?;

	Ok(NoContent)
}

#[derive(Debug, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct UnbanRequestPayload
{
	reason: UnbanReason,
}

/// Unban a player.
async fn revert(
	session: Session,
	State(svc): State<BanService>,
	Path(ban_id): Path<BanID>,
	Json(req): Json<UnbanRequestPayload>,
) -> Result<Json<UnbanResponse>>
{
	let req = UnbanRequest { ban_id, reason: req.reason, admin_id: session.user().steam_id() };
	let res = svc.unban_player(req).await?;

	Ok(Json(res))
}
