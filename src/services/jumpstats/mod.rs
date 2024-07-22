//! A service for managing jumpstats.

/* TODO:
 * - run new submissions through anti-cheat service
 * - include replays in submitted jumpstats
 *    - allow downloading the replay for a jumpstat
 */

use axum::extract::FromRef;
use sqlx::{MySql, Pool, QueryBuilder};
use tap::{Conv, Tap};

mod error;
pub use error::{Error, Result};

mod models;
pub use models::{
	FetchJumpstatRequest,
	FetchJumpstatResponse,
	FetchJumpstatsRequest,
	FetchJumpstatsResponse,
	JumpstatID,
	SubmitJumpstatRequest,
	SubmitJumpstatResponse,
};

use crate::database::{FilteredQueryBuilder, QueryBuilderExt, SqlErrorExt, TransactionExt};
use crate::services::AuthService;

mod queries;
mod http;

/// A service for managing jumpstats.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct JumpstatService
{
	database: Pool<MySql>,
	auth_svc: AuthService,
}

impl JumpstatService
{
	/// Create a new [`JumpstatService`].
	pub fn new(database: Pool<MySql>, auth_svc: AuthService) -> Self
	{
		Self { database, auth_svc }
	}

	/// Fetch a jumpstat.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_jumpstat(
		&self,
		req: FetchJumpstatRequest,
	) -> Result<Option<FetchJumpstatResponse>>
	{
		let mut query = QueryBuilder::new(queries::SELECT).tap_mut(|query| {
			query.push(" WHERE j.id = ").push_bind(req.jumpstat_id);
			query.push_limits(1, 0);
		});

		let jumpstat = query
			.build_query_as::<FetchJumpstatResponse>()
			.fetch_optional(&self.database)
			.await?;

		Ok(jumpstat)
	}

	/// Fetch jumpstats.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn fetch_jumpstats(
		&self,
		req: FetchJumpstatsRequest,
	) -> Result<FetchJumpstatsResponse>
	{
		let mut txn = self.database.begin().await?;
		let mut query = FilteredQueryBuilder::new(queries::SELECT);

		if let Some(jump_type) = req.jump_type {
			query.filter(" j.type = ", jump_type);
		}

		if let Some(mode) = req.mode {
			query.filter(" j.mode = ", mode);
		}

		if let Some(minimum_distance) = req.minimum_distance {
			query.filter(" j.distance >= ", minimum_distance);
		}

		if let Some(player) = req.player {
			let player_id = player.resolve_id(txn.as_mut()).await?;

			query.filter(" p.id = ", player_id);
		}

		if let Some(server) = req.server {
			let server_id = server.resolve_id(txn.as_mut()).await?;

			query.filter(" s.id = ", server_id);
		}

		if let Some(created_after) = req.created_after {
			query.filter(" j.created_on > ", created_after);
		}

		if let Some(created_before) = req.created_before {
			query.filter(" j.created_on < ", created_before);
		}

		query.push_limits(*req.limit, *req.offset);

		let jumpstats = query
			.build_query_as::<FetchJumpstatResponse>()
			.fetch_all(txn.as_mut())
			.await?;

		let total = txn.total_rows().await?;

		txn.commit().await?;

		Ok(FetchJumpstatsResponse { jumpstats, total })
	}

	/// Submit a new jumpstat.
	#[tracing::instrument(skip(self), err(level = "debug"))]
	pub async fn submit_jumpstat(
		&self,
		req: SubmitJumpstatRequest,
	) -> Result<SubmitJumpstatResponse>
	{
		let mut txn = self.database.begin().await?;

		let jumpstat_id = sqlx::query! {
			r#"
			INSERT INTO
			  Jumpstats (
			    type,
			    mode_id,
			    strafes,
			    distance,
			    sync,
			    pre,
			    max,
			    overlap,
			    bad_angles,
			    dead_air,
			    height,
			    airpath,
			    deviation,
			    average_width,
			    airtime,
			    player_id,
			    server_id,
			    plugin_version_id
			  )
			VALUES
			  (
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?,
			    ?
			  )
			"#,
			req.jump_type,
			req.mode,
			req.strafes,
			req.distance,
			req.sync,
			req.pre,
			req.max,
			req.overlap,
			req.bad_angles,
			req.dead_air,
			req.height,
			req.airpath,
			req.deviation,
			req.average_width,
			req.airtime.as_secs_f64(),
			req.player_id,
			req.server_id,
			req.server_plugin_version_id,
		}
		.execute(txn.as_mut())
		.await
		.map_err(|error| {
			if error.is_fk_violation("player_id") {
				Error::PlayerDoesNotExist { steam_id: req.player_id }
			} else {
				Error::from(error)
			}
		})?
		.last_insert_id()
		.conv::<JumpstatID>();

		txn.commit().await?;

		tracing::trace!(%jumpstat_id, "submitted new jumpstat");

		Ok(SubmitJumpstatResponse { jumpstat_id })
	}
}
