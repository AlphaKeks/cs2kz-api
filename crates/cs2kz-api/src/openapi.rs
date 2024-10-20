use axum::Router;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::openapi::OpenApi;
use utoipa::Modify;
use utoipa_swagger_ui::SwaggerUi;

static DESCRIPTION: &str = "\
This is the API documentation for the backend of CS2KZ, a Counter-Strike 2 community gamemode.

It is intended to be consumed by CS2 servers running the [cs2kz-metamod] plugin, as well as our
official websites:

- <https://cs2kz.org>
- <https://dashboard.cs2kz.org>
- <https://docs.cs2kz.org>

But feel free to use it for your own purposes as well! We also have a [Discord] server.

# Responses

Not every possible response is documented on every endpoint.

Success responses are always documented, as well as any special error conditions.

Generic errors like `400 Bad Request` are implied on endpoints that accept input (e.g. URI path
parameters), and as such are not documented explicitly. The most common statuses include:

- `400 Bad Request` on malformed URI input
- `401 Unauthorized` on protected endpoints
- `404 Not Found` on unknown URIs or when a specific resource could not be found
  (e.g. `/users/<non-existent ID>`)
- `409 Conflict` on protected endpoints with specific invariants (these are documented per-endpoint)
- `422 Unprocessable Entity` on malformed request body

A `500 Internal Server Error` response is always considered a bug, and reports are appreciated!

We use [RFC 7807: Problem Details for HTTP APIs][rfc7807] for our error responses, so if you
receive a status code in the `4xx` or `5xx` range, you should decode the response body according to
that format. An exhaustive list of \"problems\" (as described by the RFC), in machine-readable
format, can be found at `/docs/problems`.

Every response also includes an `x-request-id` header, which is useful to include in bug reports.

# Development

The source code for this project is available on [GitHub][github].

[cs2kz-metamod]: https://github.com/KZGlobalTeam/cs2kz-metamod
[MetaMod]: https://www.metamodsource.net
[Discord]: https://discord.gg/csgokz
[rfc7807]: https://datatracker.ietf.org/doc/html/rfc7807
[github]: https://github.com/KZGlobalTeam/cs2kz-api
";

#[derive(utoipa::OpenApi)]
#[openapi(
	info(
		title = "CS2KZ API",
		description = DESCRIPTION,
		contact(name = "AlphaKeks", email = "alphakeks@dawn.sh",),
		license(name = "GPL-3.0", url = "https://www.gnu.org/licenses/gpl-3.0.txt",)
	),
	modifiers(&SecurityAddon),
	components(schemas(
		crate::users::permissions::Permissions,
	)),
	paths(
		crate::users::http::get_users,
		crate::users::http::get_current_user,
		// crate::users::http::get_user,
		// crate::users::http::update_user,
		// crate::users::sessions::http::get_sessions,
		// crate::users::sessions::http::get_current_session,
	)
)]
pub struct Schema;

struct SecurityAddon;

impl Schema {
	pub fn generate() -> OpenApi {
		<Self as utoipa::OpenApi>::openapi()
	}

	pub fn json() -> String {
		Self::generate()
			.to_pretty_json()
			.expect("schema should be valid JSON")
	}
}

pub fn swagger_ui<S>() -> Router<S>
where
	S: Clone + Send + Sync + 'static,
{
	let schema = Schema::generate();
	let config = utoipa_swagger_ui::Config::from("/docs/openapi.json")
		.display_operation_id(true)
		.use_base_layout()
		.try_it_out_enabled(true);

	let swagger_ui = SwaggerUi::new("/docs/swagger-ui")
		.url("/docs/openapi.json", schema)
		.config(config);

	swagger_ui.into()
}

impl Modify for SecurityAddon {
	fn modify(&self, openapi: &mut OpenApi) {
		let security_schemes = &mut openapi
			.components
			.get_or_insert_with(Default::default)
			.security_schemes;

		security_schemes.insert(
			String::from("session"),
			SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("kz-auth"))),
		);
	}
}
