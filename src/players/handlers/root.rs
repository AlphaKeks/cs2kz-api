//! Handlers for the `/players` route.

use axum::Json;
use axum_extra::extract::Query;
use futures::TryStreamExt;
use serde::Deserialize;
use sqlx::QueryBuilder;
use tracing::info;
use utoipa::IntoParams;

use crate::authentication::Jwt;
use crate::authorization::Permissions;
use crate::database::{query, QueryBuilderExt, SqlxErrorExt};
use crate::http::{HandlerError, HandlerResult, Pagination};
use crate::openapi::parameters::{Limit, Offset};
use crate::openapi::responses::{self, Created};
use crate::players::{queries, FullPlayer, NewPlayer};
use crate::{authentication, authorization, State};

/// Query parameters for `GET /players`.
#[derive(Debug, Clone, Copy, Deserialize, IntoParams)]
pub struct GetParams {
	/// Limit the number of returned results.
	#[serde(default)]
	limit: Limit,

	/// Paginate by `offset` entries.
	#[serde(default)]
	offset: Offset,
}

/// Fetch players.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  get,
  tag = "Players",
  path = "/players",
  params(GetParams),
  responses(
    responses::Ok<Pagination<FullPlayer>>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn get(
	state: &State,
	session: Option<
		authentication::Session<authorization::HasPermissions<{ Permissions::BANS.value() }>>,
	>,
	Query(GetParams { limit, offset }): Query<GetParams>,
) -> HandlerResult<Json<Pagination<FullPlayer>>> {
	let mut transaction = state.database.begin().await?;
	let mut query = QueryBuilder::new(queries::SELECT);

	query.push_limits(limit, offset);

	let players = query
		.build_query_as::<FullPlayer>()
		.fetch(transaction.as_mut())
		.map_ok(|player| FullPlayer {
			// Only include IP addresses if the user is logged in and has permissions
			// to view them.
			ip_address: session.as_ref().and(player.ip_address),
			..player
		})
		.try_collect::<Vec<_>>()
		.await?;

	if players.is_empty() {
		return Err(HandlerError::no_content());
	}

	let total = query::total_rows(&mut transaction).await?;

	transaction.commit().await?;

	Ok(Json(Pagination::new(total, players)))
}

/// Register a new player.
///
/// This endpoint is called by CS2 servers when an unknown player joins.
#[tracing::instrument(level = "debug", skip(state))]
#[utoipa::path(
  post,
  tag = "Players",
  path = "/players",
  security(("CS2 Server" = [])),
  request_body = NewPlayer,
  responses(
    responses::Created,
    responses::BadRequest,
    responses::Unauthorized,
    responses::UnprocessableEntity,
  ),
)]
pub async fn post(
	state: &State,
	Jwt { claims: server, .. }: Jwt<authentication::Server>,
	Json(NewPlayer {
		name,
		steam_id,
		ip_address,
	}): Json<NewPlayer>,
) -> HandlerResult<Created> {
	sqlx::query! {
		r#"
		INSERT INTO
		  Players (id, name, ip_address)
		VALUES
		  (?, ?, ?)
		"#,
		steam_id,
		name,
		ip_address.to_string(),
	}
	.execute(&state.database)
	.await
	.map_err(|err| {
		if err.is_duplicate_entry() {
			HandlerError::already_exists("player").with_source(err)
		} else {
			HandlerError::from(err)
		}
	})?;

	info! {
		player.name = %name,
		player.id = %steam_id,
		server.id = %server.id(),
		"registered new player",
	};

	Ok(Created(()))
}
