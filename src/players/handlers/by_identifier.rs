//! Handlers for the `/players/{player}` route.

use std::collections::BTreeMap;

use axum::extract::Path;
use axum::Json;
use cs2kz::{PlayerIdentifier, SteamID};
use sqlx::types::Json as SqlJson;
use sqlx::{MySql, QueryBuilder, Transaction};
use tracing::trace;

use crate::authentication::Jwt;
use crate::authorization::{self, Permissions};
use crate::http::{HandlerError, HandlerResult};
use crate::maps::CourseID;
use crate::openapi::responses::Created;
use crate::openapi::{parameters, responses};
use crate::players::{
	queries, CourseSessionData, CourseSessionID, CreatedGameSession, FullPlayer, GameSessionID,
	PlayerUpdate,
};
use crate::servers::ServerID;
use crate::{authentication, State};

/// Fetch a specific player by name or SteamID.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players/{player}",
  params(parameters::PlayerIdentifier),
  responses(
    responses::Ok<FullPlayer>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: &State,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	Path(player): Path<PlayerIdentifier>,
) -> HandlerResult<Json<FullPlayer>> {
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push(" WHERE ");

	match player {
		PlayerIdentifier::SteamID(steam_id) => {
			query.push("p.id = ").push_bind(steam_id);
		}
		PlayerIdentifier::Name(name) => {
			query.push("p.name LIKE ").push_bind(format!("%{name}%"));
		}
	}

	let player = query
		.build_query_as::<FullPlayer>()
		.fetch_optional(&state.database)
		.await?
		.map(|player| FullPlayer {
			// Only include IP address if the user is logged in and has permissions to
			// view them.
			ip_address: session.as_ref().and(player.ip_address),
			..player
		})
		.ok_or_else(|| HandlerError::no_content())?;

	Ok(Json(player))
}

/// Update information about a player.
///
/// This will be used by CS2 servers.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  patch,
  tag = "Players",
  path = "/players/{player}",
  params(parameters::SteamID),
  request_body = PlayerUpdate,
  responses(
    responses::Created<CreatedGameSession>,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn patch(
	state: &State,
	Jwt { claims: server, .. }: Jwt<authentication::Server>,
	Path(steam_id): Path<SteamID>,
	Json(PlayerUpdate {
		name,
		ip_address,
		preferences,
		session_data,
	}): Json<PlayerUpdate>,
) -> HandlerResult<Created<Json<CreatedGameSession>>> {
	let mut transaction = state.database.begin().await?;

	let was_updated = sqlx::query! {
		r#"
		UPDATE
		  Players
		SET
		  name = ?,
		  ip_address = ?,
		  game_preferences = ?
		WHERE
		  id = ?
		"#,
		name,
		ip_address.to_string(),
		SqlJson(&preferences),
		steam_id,
	}
	.execute(transaction.as_mut())
	.await
	.map(|result| result.rows_affected() > 0)?;

	if !was_updated {
		return Err(HandlerError::unknown("player"));
	}

	trace! {
		player.id = %steam_id,
		player.name = %name,
		server.id = %server.id(),
		"updated player",
	};

	let game_session_id: GameSessionID = sqlx::query! {
		r#"
		INSERT INTO
		  GameSessions (
		    player_id,
		    server_id,
		    time_active,
		    time_spectating,
		    time_afk
		  )
		VALUES
		  (?, ?, ?, ?, ?)
		"#,
		steam_id,
		server.id(),
		session_data.time_active,
		session_data.time_spectating,
		session_data.time_afk,
	}
	.execute(transaction.as_mut())
	.await?
	.last_insert_id()
	.into();

	trace! {
		player.id = %steam_id,
		player.name = %name,
		server.id = %server.id(),
		game_session.id = %game_session_id,
		"inserted game session",
	};

	let mut course_session_ids = BTreeMap::new();

	for (course_id, session_data) in session_data.course_sessions {
		let course_session_id = insert_course_session_data(
			steam_id,
			course_id,
			server.id(),
			session_data,
			&mut transaction,
		)
		.await?;

		course_session_ids.insert(course_id, course_session_id);
	}

	Ok(Created(Json(CreatedGameSession {
		game_session_id,
		course_session_ids,
	})))
}

/// Inserts a single [course session] into the database and returns its ID.
///
/// [course session]: CourseSessionData
async fn insert_course_session_data(
	player_id: SteamID,
	course_id: CourseID,
	server_id: ServerID,
	CourseSessionData {
		playtime,
		started_runs,
		finished_runs,
	}: CourseSessionData,
	transaction: &mut Transaction<'_, MySql>,
) -> sqlx::Result<CourseSessionID> {
	let course_session_id = sqlx::query! {
		r#"
		INSERT INTO
		  CourseSessions (
		    player_id,
		    course_id,
		    server_id,
		    playtime,
		    started_runs,
		    finished_runs
		  )
		VALUES
		  (?, ?, ?, ?, ?, ?)
		"#,
		player_id,
		course_id,
		server_id,
		playtime,
		started_runs,
		finished_runs,
	}
	.execute(transaction.as_mut())
	.await?
	.last_insert_id()
	.into();

	trace! {
		player.id = %player_id,
		server.id = %server_id,
		course_session.id = %course_session_id,
		"inserted course session",
	};

	Ok(course_session_id)
}
