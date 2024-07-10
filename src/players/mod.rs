//! Everything related to KZ players.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::iter;
use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::FromRef;
use cs2kz::{Mode, SteamID};
use futures::{TryFutureExt, TryStreamExt};
use serde_json::Value as JsonValue;
use sqlx::types::Json as SqlJson;
use sqlx::{MySql, Pool, QueryBuilder};

use crate::authentication::JwtState;
use crate::game_sessions::{CourseSessionID, GameSessionID};
use crate::kz::PlayerIdentifier;
use crate::maps::CourseID;
use crate::servers::ServerID;
use crate::sqlx::{query, FetchID, QueryBuilderExt, SqlErrorExt};
use crate::{steam, Error, Result};

#[cfg(test)]
mod tests;

mod models;
pub use models::{
	CourseSession,
	CourseSessions,
	FetchPlayersRequest,
	FullPlayer,
	NewPlayer,
	Player,
	PlayerUpdate,
	Session,
};

mod queries;
pub mod http;

/// A service for dealing with KZ players as a resource.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct PlayerService
{
	database: Pool<MySql>,
	jwt_state: Arc<JwtState>,
	http_client: reqwest::Client,
	api_config: Arc<crate::Config>,
}

impl PlayerService
{
	/// Creates a new [`PlayerService`] instance.
	pub const fn new(
		database: Pool<MySql>,
		jwt_state: Arc<JwtState>,
		http_client: reqwest::Client,
		api_config: Arc<crate::Config>,
	) -> Self
	{
		Self { database, jwt_state, http_client, api_config }
	}

	/// Fetches a single player.
	///
	/// If `include_ip_address` is `false`, the resulting
	/// [`FullPlayer::ip_address`] field will be [`None`].
	pub async fn fetch_player(
		&self,
		player: PlayerIdentifier,
		include_ip_address: bool,
	) -> Result<FullPlayer>
	{
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push(" WHERE ");

		match player {
			PlayerIdentifier::ID(steam_id) => {
				query.push(" p.id = ").push_bind(steam_id);
			}
			PlayerIdentifier::Name(name) => {
				query.push(" p.name LIKE ").push_bind(format!("%{name}%"));
			}
		}

		let mut player = query
			.build_query_as::<FullPlayer>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("player"))?;

		// Filter out IP address if we're not in a test and the user does not have
		// permission to view IP addresses
		if cfg!(not(test)) && !include_ip_address {
			player.ip_address = None;
		}

		Ok(player)
	}

	/// Fetches many players.
	///
	/// The `limit` and `offset` fields in [`FetchPlayersRequest`] can be used
	/// for pagination. The `u64` part of the returned tuple indicates how many
	/// players _could_ be fetched; also useful for pagination.
	///
	/// If `include_ip_address` is `false`, the resulting
	/// [`FullPlayer::ip_address`] fields will be [`None`].
	pub async fn fetch_players(
		&self,
		request: FetchPlayersRequest,
		include_ip_addresses: bool,
	) -> Result<(Vec<FullPlayer>, u64)>
	{
		let mut transaction = self.database.begin().await?;
		let mut query = QueryBuilder::new(queries::SELECT);

		query.push_limits(request.limit, request.offset);

		let players = query
			.build_query_as::<FullPlayer>()
			.fetch(transaction.as_mut())
			.map_ok(|player| FullPlayer {
				// Only include IP address information if the requesting user has
				// permission to view them.
				ip_address: if cfg!(test) || include_ip_addresses {
					player.ip_address
				} else {
					None
				},

				..player
			})
			.try_collect::<Vec<_>>()
			.await?;

		if players.is_empty() {
			return Err(Error::no_content());
		}

		let total = query::total_rows(&mut transaction).await?;

		transaction.commit().await?;

		Ok((players, total))
	}

	/// Registers a new player.
	///
	/// This will return an error if the player already exists!
	pub async fn register_player(&self, player: NewPlayer) -> Result<()>
	{
		sqlx::query! {
			r#"
			INSERT INTO
			  Players (id, name, ip_address)
			VALUES
			  (?, ?, ?)
			"#,
			player.steam_id,
			player.name,
			match player.ip_address {
				IpAddr::V4(ip) => ip.to_ipv6_mapped(),
				IpAddr::V6(ip) => ip,
			},
		}
		.execute(&self.database)
		.await
		.map_err(|err| {
			if err.is_duplicate_entry() {
				Error::already_exists("player").context(err)
			} else {
				Error::from(err)
			}
		})?;

		tracing::info!(target: "cs2kz_api::audit_log", "registered new player");

		Ok(())
	}

	/// Updates an existing player.
	///
	/// Player updates should only ever be sent by servers. The `server_id`
	/// parameter indicates which server this update is coming from.
	pub async fn update_player(
		&self,
		player_id: SteamID,
		server_id: ServerID,
		update: PlayerUpdate,
	) -> Result<()>
	{
		let mut transaction = self.database.begin().await?;

		let query_result = sqlx::query! {
			r#"
			UPDATE
			  Players
			SET
			  name = ?,
			  ip_address = ?,
			  preferences = ?,
			  last_seen_on = NOW()
			WHERE
			  id = ?
			"#,
			update.name,
			match update.ip_address {
				IpAddr::V4(ip) => ip.to_ipv6_mapped(),
				IpAddr::V6(ip) => ip,
			},
			SqlJson(&update.preferences),
			player_id,
		}
		.execute(transaction.as_mut())
		.await?;

		match query_result.rows_affected() {
			0 => return Err(Error::not_found("player")),
			n => assert_eq!(n, 1, "updated more than 1 player"),
		}

		tracing::trace!(target: "cs2kz_api::audit_log", "updated player");

		let session_id: GameSessionID = sqlx::query! {
			r#"
			INSERT INTO
			  GameSessions (
			    player_id,
			    server_id,
			    time_active,
			    time_spectating,
			    time_afk,
			    bhops,
			    perfs
			  )
			VALUES
			  (?, ?, ?, ?, ?, ?, ?)
			"#,
			player_id,
			server_id,
			update.session.time_spent.active.as_secs(),
			update.session.time_spent.spectating.as_secs(),
			update.session.time_spent.afk.as_secs(),
			update.session.bhop_stats.bhops,
			update.session.bhop_stats.perfs,
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

		tracing::trace!(target: "cs2kz_api::audit_log", %session_id, "created game session");

		let mut course_session_ids = Vec::with_capacity(update.session.course_sessions.len());

		for (course_id, (mode, session)) in update
			.session
			.course_sessions
			.into_iter()
			.flat_map(|(course_id, sessions)| iter::zip(iter::repeat(course_id), sessions))
		{
			insert_course_session(player_id, server_id, course_id, mode, session, &mut transaction)
				.map_ok(|id| course_session_ids.push(id))
				.await?;
		}

		tracing::trace!(target: "cs2kz_api::audit_log", ?course_session_ids, "created course sessions");

		transaction.commit().await?;

		Ok(())
	}

	/// Fetches a player's Steam profile.
	pub async fn fetch_steam_profile(&self, player: PlayerIdentifier) -> Result<steam::User>
	{
		let steam_id = player.fetch_id(&self.database).await?;
		let user = steam::User::fetch(steam_id, &self.http_client, &self.api_config).await?;

		Ok(user)
	}

	/// Fetches a player's in-game preferences.
	pub async fn fetch_preferences(&self, player: PlayerIdentifier) -> Result<JsonValue>
	{
		let mut query = QueryBuilder::new("SELECT preferences FROM Players WHERE");

		match player {
			PlayerIdentifier::ID(steam_id) => {
				query.push(" id = ").push_bind(steam_id);
			}
			PlayerIdentifier::Name(name) => {
				query.push(" name LIKE ").push_bind(format!("%{name}%"));
			}
		}

		let SqlJson(preferences) = query
			.build_query_scalar::<SqlJson<JsonValue>>()
			.fetch_optional(&self.database)
			.await?
			.ok_or_else(|| Error::not_found("player"))?;

		Ok(preferences)
	}
}

/// Inserts a [`CourseSession`] into the database and returns the generated
/// [`CourseSessionID`].
async fn insert_course_session(
	steam_id: SteamID,
	server_id: ServerID,
	course_id: CourseID,
	mode: Mode,
	CourseSession { playtime, started_runs, finished_runs, bhop_stats }: CourseSession,
	transaction: &mut sqlx::Transaction<'_, MySql>,
) -> Result<CourseSessionID>
{
	let session_id = sqlx::query! {
		r#"
		INSERT INTO
		  CourseSessions (
		    player_id,
		    course_id,
		    mode_id,
		    server_id,
		    playtime,
		    started_runs,
		    finished_runs,
		    bhops,
		    perfs
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?, ?, ?, ?)
		"#,
		steam_id,
		course_id,
		mode,
		server_id,
		playtime,
		started_runs,
		finished_runs,
		bhop_stats.bhops,
		bhop_stats.perfs,
	}
	.execute(transaction.as_mut())
	.await
	.map_err(|err| {
		if err.is_fk_violation_of("player_id") {
			Error::not_found("player").context(err)
		} else if err.is_fk_violation_of("course_id") {
			Error::not_found("course").context(err)
		} else {
			Error::from(err)
		}
	})?
	.last_insert_id()
	.into();

	tracing::trace! {
		target: "cs2kz_api::audit_log",
		%course_id,
		%session_id,
		"created course session",
	};

	Ok(session_id)
}
