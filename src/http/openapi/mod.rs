use std::sync::OnceLock;

use utoipa::openapi::OpenApi;

use self::modifiers::SecurityAddon;

mod modifiers;

pub(crate) static SCHEMA: OnceLock<OpenApi> = OnceLock::new();

#[derive(utoipa::OpenApi)]
#[openapi(
    info(
        title = "CS2KZ API",
        description = "",
        license(name = "GPL-3.0", url = "https://www.gnu.org/licenses/gpl-3.0.en.html"),
    ),
    external_docs(url = "https://docs.cs2kz.org/api", description = "High-Level documentation"),
    servers((url = "https://api.cs2kz.org")),
    modifiers(&SecurityAddon),
    tags(
        (name = "Leaderboards"),
        (name = "Records"),
        (name = "Maps"),
        (name = "Servers"),
        (name = "Bans"),
        (name = "Players"),
        (name = "Users"),
        (name = "Mappers"),
        (name = "User Authentication", description = "OpenID 2.0 authentication with Steam"),
        (name = "Events", description = "Real-Time events via SSE"),
        (name = "Plugin", description = "GOKZ/cs2kz-metamod"),
    ),
    components(
        schemas(
			crate::http::pagination::Offset,
			crate::http::pagination::Limit<10, 1000>,
			crate::http::pagination::Limit<100, 1000>,
			crate::http::pagination::Limit<1000, 1000>,

			cs2kz_api::mode::Mode,
			cs2kz_api::maps::MapState,
			cs2kz_api::records::Leaderboard,
        ),
    ),
    paths(
		crate::http::handlers::get_rating_leaderboard,
		crate::http::handlers::get_records_leaderboard,
		crate::http::handlers::get_course_leaderboard,

		crate::http::handlers::get_records,
		crate::http::handlers::get_record,

		crate::http::handlers::create_map,
		crate::http::handlers::get_maps,
		crate::http::handlers::get_map,
		crate::http::handlers::update_map,
		crate::http::handlers::update_map_state,

		crate::http::handlers::create_server,
		crate::http::handlers::get_servers,
		crate::http::handlers::get_server,
		crate::http::handlers::update_server,
		crate::http::handlers::reset_server_access_key,
		crate::http::handlers::delete_server_access_key,

		crate::http::handlers::create_ban,
		crate::http::handlers::get_bans,
		crate::http::handlers::get_ban,
		crate::http::handlers::update_ban,
		crate::http::handlers::revert_ban,

		crate::http::handlers::get_players,
		crate::http::handlers::get_player,
		crate::http::handlers::get_player_preferences,
		crate::http::handlers::update_player_preferences,

		crate::http::handlers::get_users,
		crate::http::handlers::get_user,
		crate::http::handlers::update_user_email,
		crate::http::handlers::delete_user_email,
		crate::http::handlers::update_user_permissions,
		crate::http::handlers::update_user_server_budget,

		crate::http::handlers::create_mapper,
		crate::http::handlers::delete_mapper,

		crate::http::handlers::web_login,
		crate::http::handlers::web_logout,

		crate::http::handlers::events,

		crate::http::handlers::create_plugin_version,
		crate::http::handlers::get_plugin_versions,
    ),
)]
pub(crate) struct Schema;

pub(crate) fn schema() -> OpenApi
{
	<Schema as utoipa::OpenApi>::openapi()
}
