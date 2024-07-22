//! HTTP handlers for this service.

use axum::extract::{Path, State};
use axum::{routing, Json, Router};
use axum_extra::extract::Query;
use cs2kz::{JumpType, Mode, SteamID};
use serde::Deserialize;
use tower::ServiceBuilder;

use super::{
	Error,
	FetchJumpstatResponse,
	FetchJumpstatsRequest,
	FetchJumpstatsResponse,
	JumpstatID,
	JumpstatService,
	SubmitJumpstatRequest,
	SubmitJumpstatResponse,
};
use crate::middleware;
use crate::runtime::Result;
use crate::services::auth::jwt::JwtLayer;
use crate::services::auth::{self, Jwt};
use crate::services::jumpstats::FetchJumpstatRequest;
use crate::util::time::Seconds;

impl From<JumpstatService> for Router
{
	fn from(svc: JumpstatService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer)
			.layer(JwtLayer::<auth::ServerInfo>::new(svc.auth_svc.clone()));

		Router::new()
			.route("/", routing::get(get_many))
			.route("/", routing::post(submit).route_layer(auth))
			.route("/:id", routing::get(get_single))
			.with_state(svc)
	}
}

/// Fetch a jumpstat by its ID.
async fn get_single(
	State(svc): State<JumpstatService>,
	Path(jumpstat_id): Path<JumpstatID>,
) -> Result<Json<FetchJumpstatResponse>>
{
	let res = svc
		.fetch_jumpstat(FetchJumpstatRequest { jumpstat_id })
		.await?
		.ok_or(Error::JumpstatDoesNotExist { jumpstat_id })?;

	Ok(Json(res))
}

/// Fetch many jumpstats.
async fn get_many(
	State(svc): State<JumpstatService>,
	Query(req): Query<FetchJumpstatsRequest>,
) -> Result<Json<FetchJumpstatsResponse>>
{
	let res = svc.fetch_jumpstats(req).await?;

	if res.jumpstats.is_empty() {
		return Err(Error::NoData.into());
	}

	Ok(Json(res))
}

#[derive(Debug, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct SubmitJumpstatRequestPayload
{
	/// The jump type.
	pub jump_type: JumpType,

	/// The mode the jump was performed in.
	pub mode: Mode,

	/// The SteamID of the player who performed the jump.
	pub player_id: SteamID,

	/// How many strafes the player performed during the jump.
	pub strafes: u8,

	/// The distance cleared by the jump.
	pub distance: f32,

	/// The % of airtime spent gaining speed.
	pub sync: f32,

	/// The speed at jumpoff.
	pub pre: f32,

	/// The maximum speed during the jump.
	pub max: f32,

	/// The amount of time spent pressing both strafe keys.
	pub overlap: Seconds,

	/// The amount of time spent pressing keys but not gaining speed.
	pub bad_angles: Seconds,

	/// The amount of time spent doing nothing.
	pub dead_air: Seconds,

	/// The maximum height reached during the jump.
	pub height: f32,

	/// How close to a perfect airpath this jump was.
	///
	/// The closer to 1.0 the better.
	pub airpath: f32,

	/// How far the landing position deviates from the jumpoff position.
	pub deviation: f32,

	/// The average strafe width.
	pub average_width: f32,

	/// The amount of time spent mid-air.
	pub airtime: Seconds,
}

async fn submit(
	jwt: Jwt<auth::ServerInfo>,
	State(svc): State<JumpstatService>,
	Json(req): Json<SubmitJumpstatRequestPayload>,
) -> Result<Json<SubmitJumpstatResponse>>
{
	let req = SubmitJumpstatRequest {
		jump_type: req.jump_type,
		mode: req.mode,
		player_id: req.player_id,
		strafes: req.strafes,
		distance: req.distance,
		sync: req.sync,
		pre: req.pre,
		max: req.max,
		overlap: req.overlap,
		bad_angles: req.bad_angles,
		dead_air: req.dead_air,
		height: req.height,
		airpath: req.airpath,
		deviation: req.deviation,
		average_width: req.average_width,
		airtime: req.airtime,
		server_id: jwt.id(),
		server_plugin_version_id: jwt.plugin_version_id(),
	};

	let res = svc.submit_jumpstat(req).await?;

	Ok(Json(res))
}
