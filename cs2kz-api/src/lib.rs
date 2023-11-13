use {
	crate::state::AppState,
	axum::{routing, Router},
	utoipa::OpenApi,
	utoipa_swagger_ui::SwaggerUi,
};

pub mod error;
use color_eyre::eyre::Context;
pub use error::{Error, Result};

pub mod util;
pub mod database;

pub mod logging;
pub mod routes;
pub mod middleware;
pub mod state;
pub mod res;

#[rustfmt::skip]
#[derive(OpenApi)]
#[openapi(
	info(
		title = "CS2KZ API",
		version = "0.0.0",
		license(
			name = "License: GPLv3.0",
			url = "https://www.gnu.org/licenses/gpl-3.0",
		),
	),

	paths(
		routes::health::health,

		routes::players::get_players,
		routes::players::get_player,
		routes::players::create_player,
		routes::players::update_player,

		routes::bans::get_bans,
		routes::bans::get_replay,
		routes::bans::create_ban,

		routes::maps::get_maps,
		routes::maps::get_map,
		routes::maps::create_map,
		routes::maps::update_map,

		routes::servers::get_servers,
		routes::servers::get_server,
		routes::servers::create_server,
		routes::servers::update_server,

		routes::records::get_records,
		routes::records::get_record,
		routes::records::get_replay,
		routes::records::create_record,
	),

	components(
		schemas(
			cs2kz::SteamID,
			cs2kz::PlayerIdentifier,
			cs2kz::MapIdentifier,
			cs2kz::ServerIdentifier,
			cs2kz::Mode,
			cs2kz::Style,
			cs2kz::Jumpstat,
			cs2kz::Tier,
			cs2kz::Runtype,

			crate::Error,

			crate::res::PlayerInfo,

			crate::res::player::Player,
			crate::routes::players::NewPlayer,
			crate::routes::players::PlayerUpdate,

			crate::res::bans::Ban,
			crate::res::bans::BanReason,
			crate::routes::bans::NewBan,
			crate::routes::bans::CreatedBan,

			crate::res::maps::KZMap,
			crate::res::maps::MapCourse,
			crate::routes::maps::NewMap,
			crate::routes::maps::Course,
			crate::routes::maps::Filter,
			crate::routes::maps::CreatedMap,
			crate::routes::maps::CreatedCourse,
			crate::routes::maps::CreatedFilter,
			crate::routes::maps::MapUpdate,
			crate::routes::maps::FilterWithCourseId,

			crate::res::servers::Server,
			crate::routes::servers::NewServer,
			crate::routes::servers::CreatedServer,
			crate::routes::servers::ServerUpdate,

			crate::res::records::Record,
			crate::res::records::RecordMap,
			crate::res::records::RecordCourse,
			crate::res::records::RecordPlayer,
			crate::res::records::RecordServer,
			crate::routes::records::NewRecord,
			crate::routes::records::CreatedRecord,
		),

		responses(
			res::BadRequest,
		),
	),
)]
pub struct API;

impl API {
	/// Creates an [`axum::Router`] which will be used by the HTTP server.
	pub fn router(state: AppState) -> Router {
		let state: &'static AppState = Box::leak(Box::new(state));

		let public_api_router = Router::new()
			.route("/health", routing::get(routes::health::health))
			.route("/players", routing::get(routes::players::get_players))
			.route("/players/:ident", routing::get(routes::players::get_player))
			.route("/bans", routing::get(routes::bans::get_bans))
			.route("/bans/:id/replay", routing::get(routes::bans::get_replay))
			.route("/maps", routing::get(routes::maps::get_maps))
			.route("/maps/:ident", routing::get(routes::maps::get_map))
			.route("/servers", routing::get(routes::servers::get_servers))
			.route("/servers/:ident", routing::get(routes::servers::get_server))
			.route("/records", routing::get(routes::records::get_records))
			.route("/record/:id", routing::get(routes::records::get_record))
			.route("/record/:id/replay", routing::get(routes::records::get_replay))
			.with_state(state);

		let game_server_auth =
			axum::middleware::from_fn_with_state(state, middleware::auth::gameservers::auth_server);

		// Routes to be used by cs2kz servers (require auth).
		let game_server_router = Router::new()
			.route("/players", routing::post(routes::players::create_player))
			.route("/players/:ident", routing::put(routes::players::update_player))
			.route("/bans", routing::post(routes::bans::create_ban))
			.route("/record", routing::post(routes::records::create_record))
			.layer(game_server_auth)
			.with_state(state);

		// let map_approval_router = Router::new()
		// 	.route("/maps", routing::post(routes::maps::create_map))
		// 	.route("/maps/:ident", routing::put(routes::maps::update_map))
		// 	.with_state(state);

		// let server_approval_router = Router::new()
		// 	.route("/servers", routing::post(routes::servers::create_server))
		// 	.route("/servers/:ident", routing::put(routes::servers::update_server))
		// 	.with_state(state);

		let api_router = game_server_router.merge(public_api_router);

		let swagger_ui = Self::swagger_ui();

		Router::new()
			.nest("/api/v0", api_router)
			.merge(swagger_ui)
	}

	/// Creates an iterator over all of the API's routes.
	pub fn routes() -> impl Iterator<Item = String> {
		Self::openapi().paths.paths.into_keys()
	}

	/// Saves a JSON version of the OpenAPI spec on disk.
	pub fn write_json() -> color_eyre::Result<()> {
		let json = Self::openapi()
			.to_pretty_json()
			.context("Failed to convert API to JSON.")?;

		std::fs::write("api-spec.json", json.into_bytes())
			.context("Failed to write JSON to disk.")?;

		Ok(())
	}

	/// Creates a tower service layer for serving an HTML page with SwaggerUI.
	pub fn swagger_ui() -> SwaggerUi {
		SwaggerUi::new("/api/docs/swagger-ui").url("/api/docs/openapi.json", Self::openapi())
	}
}

/// Type alias for easy use in function signatures.
///
/// You can read more about axum's extractors [here](https://docs.rs/axum/0.6.20/axum/index.html#extractors).
///
/// Usually you would write a handler function like this:
///
/// ```ignore
/// use axum::extract::State;
/// use crate::State as AppState;
///
/// async fn handler(State(state): State<&'static AppState>) {
///     let db = state.database();
///     // ...
/// }
/// ```
///
/// To avoid all that type "boilerplate", you can use this type alias instead:
///
/// ```ignore
/// async fn handler(state: crate::State) {
///     let db = state.database();
///     // ...
/// }
/// ```
pub type State = axum::extract::State<&'static crate::AppState>;
