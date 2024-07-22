//! HTTP handlers for this service.

use axum::extract::{Path, State};
use axum::{routing, Json, Router};
use axum_extra::extract::Query;
use cs2kz::SteamID;
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	AdminService,
	Error,
	FetchAdminRequest,
	FetchAdminResponse,
	FetchAdminsRequest,
	FetchAdminsResponse,
	SetPermissionsRequest,
};
use crate::middleware;
use crate::runtime::Result;
use crate::services::auth::session::authorization::RequiredPermissions;
use crate::services::auth::session::{user, SessionManagerLayer};

impl From<AdminService> for Router
{
	fn from(svc: AdminService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer)
			.layer(SessionManagerLayer::with_authorization(
				svc.auth_svc.clone(),
				RequiredPermissions(user::Permissions::ADMIN),
			));

		Router::new()
			.route("/", routing::get(get_many))
			.route("/:steam_id", routing::get(get_single))
			.route("/:steam_id", routing::put(set_permissions).route_layer(auth))
			.with_state(svc)
	}
}

/// Fetch a specific ban by its ID.
async fn get_single(
	State(svc): State<AdminService>,
	Path(user_id): Path<SteamID>,
) -> Result<Json<FetchAdminResponse>>
{
	let res = svc
		.fetch_admin(FetchAdminRequest { user_id })
		.await?
		.ok_or(super::Error::UserDoesNotExist { user_id })?;

	Ok(Json(res))
}

/// Fetch many bans.
async fn get_many(
	State(svc): State<AdminService>,
	Query(req): Query<FetchAdminsRequest>,
) -> Result<Json<FetchAdminsResponse>>
{
	let res = svc.fetch_admins(req).await?;

	if res.admins.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(Json(res))
}

/// Query parameters for the `set_permissions` handler.
#[derive(Debug, Deserialize)]
pub struct SetPermissionsQuery
{
	/// The permissions to set for the user.
	permissions: user::Permissions,
}

/// Set a user's permissions.
async fn set_permissions(
	State(svc): State<AdminService>,
	Path(user_id): Path<SteamID>,
	Query(req): Query<SetPermissionsQuery>,
) -> Result<()>
{
	svc.set_permissions(SetPermissionsRequest { user_id, permissions: req.permissions })
		.await?;

	Ok(())
}
