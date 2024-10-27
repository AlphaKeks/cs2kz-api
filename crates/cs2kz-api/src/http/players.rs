use axum::extract::FromRef;
use axum::{Router, routing};
use futures_util::TryStreamExt;

use crate::database;
use crate::http::extract::{Json, Path, Query};
use crate::http::response::{ErrorResponse, NotFound};
use crate::pagination::{Limit, Offset, PaginationResult, TryStreamExt as _};
use crate::players::{self, Player, PlayerID, Preferences};
use crate::users::Permission;
use crate::users::sessions::Session;

pub fn router<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
	database::ConnectionPool: FromRef<S>,
{
	Router::new()
		.route("/", routing::get(get_players))
		.route("/{player}", routing::get(get_player))
		.route(
			"/{player}/preferences",
			routing::get(get_player_preferences),
		)
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
struct GetPlayersQuery {
	#[serde(default)]
	limit: Limit<1000, 100>,

	#[serde(default)]
	offset: Offset,
}

/// Returns information about KZ players.
///
/// IP addresses will be included if you're allowed to view them.
#[instrument]
#[utoipa::path(get, path = "/players", tag = "Players", params(GetPlayersQuery), responses(
	(status = 200, body = [Player]),
))]
pub(crate) async fn get_players(
	session: Option<Session>,
	mut db_conn: database::Connection,
	Query(GetPlayersQuery { limit, offset }): Query<GetPlayersQuery>,
) -> Result<PaginationResult<Player>, ErrorResponse> {
	let (total, stream) = players::get_players(&mut db_conn, limit.get(), offset.0).await?;
	let result = stream
		.map_ok(ip_filter(session.as_ref()))
		.try_collect_into_pagination_result(total, limit.max())
		.await?;

	Ok(result)
}

/// Returns information about a KZ player.
///
/// IP address will be included if you're allowed to view it.
#[instrument]
#[utoipa::path(
	get,
	path = "/players/{player}",
	tag = "Players",
	params(("player", description = "a SteamID or name")),
	responses((status = 200, body = Player)),
)]
pub(crate) async fn get_player(
	session: Option<Session>,
	mut db_conn: database::Connection,
	Path(player): Path<String>,
) -> Result<Json<Player>, ErrorResponse> {
	let player = if let Ok(player_id) = player.parse::<PlayerID>() {
		players::get_player_by_id(&mut db_conn, player_id).await?
	} else {
		players::get_player_by_name(&mut db_conn, &player).await?
	}
	.ok_or(NotFound)?;

	Ok(Json(player))
}

/// Returns a player's in-game preferences.
#[instrument]
#[utoipa::path(
	get,
	path = "/players/{player}/preferences",
	tag = "Players",
	params(("player", description = "a SteamID")),
	responses((status = 200, body = Preferences)),
)]
pub(crate) async fn get_player_preferences(
	session: Option<Session>,
	mut db_conn: database::Connection,
	Path(player_id): Path<PlayerID>,
) -> Result<Json<Preferences>, ErrorResponse> {
	let preferences = players::get_preferences(&mut db_conn, player_id)
		.await?
		.ok_or(NotFound)?;

	Ok(Json(preferences))
}

/// Creates a mapping function that filters out IP addresses if necessary.
///
/// Only users with the `Bans` permission are allowed to view player IPs, so if
/// the request is not authorized, we want to remove them.
fn ip_filter(session: Option<&Session>) -> impl Fn(Player) -> Player + use<> {
	let include_ip_addrs = session.is_some_and(|session| {
		session
			.user()
			.permissions()
			.contains_permission(Permission::Bans)
	});

	move |player| Player {
		ip_address: if include_ip_addrs {
			player.ip_address
		} else {
			None
		},
		..player
	}
}
