//! Everything related to jumpstats.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::authentication::JwtState;
use crate::sqlx::query::QueryBuilderExt;
use crate::sqlx::{query, FetchID, FilteredQuery, SqlErrorExt};
use crate::{authentication, Error, Result};

mod models;
pub use models::{CreatedJumpstat, FetchJumpstatsRequest, Jumpstat, JumpstatID, NewJumpstat};

mod queries;
pub mod http;

/// A service for dealing with KZ jumpstats as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct JumpstatService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
}

impl JumpstatService
{
	/// Creates a new [`JumpstatService`] instance.
	pub const fn new(database: Pool<MySql>, jwt_state: Arc<JwtState>) -> Self
	{
		Self { database, jwt_state }
	}

	/// Fetches a single jumpstat.
	pub async fn fetch_jumpstat(&self, jumpstat_id: JumpstatID) -> Result<Jumpstat>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE j.id = ").push_bind(jumpstat_id);

		let jumpstat = query
			.build_query_as::<Jumpstat>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("jumpstat"))?;

		Ok(jumpstat)
	}

	/// Fetches many jumpstats.
	pub async fn fetch_jumpstats(
		&self,
		request: FetchJumpstatsRequest,
	) -> Result<(Vec<Jumpstat>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = FilteredQuery::new(queries::SELECT);

		if let Some(jump_type) = request.jump_type {
			query.filter(" j.type = ", jump_type);
		}

		if let Some(mode) = request.mode {
			query.filter(" j.mode_id = ", mode);
		}

		if let Some(minimum_distance) = request.minimum_distance {
			query.filter(" j.distance >= ", minimum_distance);
		}

		if let Some(player) = request.player {
			let steam_id = player.fetch_id(transaction.as_mut()).await?;

			query.filter(" j.player_id = ", steam_id);
		}

		if let Some(server) = request.server {
			let server_id = server.fetch_id(transaction.as_mut()).await?;

			query.filter(" j.server_id = ", server_id);
		}

		if let Some(created_after) = request.created_after {
			query.filter(" j.created_on > ", created_after);
		}

		if let Some(created_before) = request.created_before {
			query.filter(" j.created_on < ", created_before);
		}

		query.push_limits(request.limit, request.offset);

		let jumpstats = query
			.build_query_as::<Jumpstat>()
			.fetch_all(transaction.as_mut())
			.await?;

		if jumpstats.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((jumpstats, total))
	}

	/// Submits a new jumpstat.
	pub async fn submit_jumpstat(
		&self,
		jumpstat: NewJumpstat,
		server: authentication::Server,
	) -> Result<CreatedJumpstat>
	{
		let mut transaction = self.database.begin().await?;

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
			jumpstat.jump_type,
			jumpstat.mode,
			jumpstat.strafes,
			jumpstat.distance,
			jumpstat.sync,
			jumpstat.pre,
			jumpstat.max,
			jumpstat.overlap,
			jumpstat.bad_angles,
			jumpstat.dead_air,
			jumpstat.height,
			jumpstat.airpath,
			jumpstat.deviation,
			jumpstat.average_width,
			jumpstat.airtime.as_secs_f64(),
			jumpstat.player_id,
			server.id(),
			server.plugin_version_id(),
		}
		.execute(transaction.as_mut())
		.await
		.map_err(|err| {
			if err.is_fk_violation_of("player_id") {
				Error::not_found("player").context(err)
			} else {
				Error::from(err)
			}
		})?
		.last_insert_id()
		.into();

		transaction.commit().await?;

		tracing::trace!(%jumpstat_id, "created jumpstat");

		Ok(CreatedJumpstat { jumpstat_id })
	}
}
