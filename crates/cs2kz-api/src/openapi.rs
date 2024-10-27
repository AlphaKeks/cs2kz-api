//! OpenAPI documentation.

use utoipa::OpenApi;
use utoipa::openapi::OpenApi as OpenApiDoc;

/// The OpenAPI schema.
#[derive(OpenApi)]
#[openapi(
	info(
		title = "CS2KZ API",
		description = "This is the API documentation for CS2KZ's backend.\n\n\
		               Feel free to join our [Discord] server!\n\n\
		               [Discord]: https://discord.gg/csgokz",
		contact(name = "AlphaKeks", email = "alphakeks@dawn.sh"),
		license(name = "GPL-3.0", url = "https://www.gnu.org/licenses/gpl-3.0.txt"),
	),
	external_docs(
		url = "https://docs.cs2kz.org",
		description = "General CS2KZ documentation"
	),
	components(schemas(
		crate::pagination::Limit<50, 10>,
		crate::pagination::Offset,
		crate::users::UserID,
		crate::users::Permissions,
	)),
	paths(
		// `/users`
		crate::http::users::get_users,
		crate::http::users::get_user_by_id,
		crate::http::users::get_sessions,
		crate::http::users::delete_sessions,
		crate::http::users::get_current_session,
		crate::http::users::delete_current_session,

		// `/plugin`
		crate::http::plugin::get_plugin_versions,
		crate::http::plugin::publish_plugin_version,
		crate::http::plugin::get_plugin_version,

		// `/players`
		crate::http::players::get_players,
		crate::http::players::get_player,
		crate::http::players::get_player_preferences,
	),
)]
pub struct Schema(OpenApiDoc);

impl Schema {
	/// Generates the OpenAPI schema.
	#[expect(clippy::new_without_default)]
	pub fn new() -> Self {
		Self(Self::openapi())
	}

	/// Returns the [`utoipa::openapi::OpenApi`] document associated with this
	/// schema.
	pub fn api_doc(&self) -> &OpenApiDoc {
		&self.0
	}
}

/// Returns the SwaggerUI configuration.
pub fn swagger_ui_config() -> utoipa_swagger_ui::Config<'static> {
	utoipa_swagger_ui::Config::from("/docs/openapi.json")
		.display_operation_id(true)
		// .use_base_layout()
		.display_request_duration(true)
		.filter(true)
		.try_it_out_enabled(true)
		.request_snippets_enabled(true)
		.with_credentials(true)
}
