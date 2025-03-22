use cs2kz_api::users::sessions::SessionId;
use utoipa::{
	Modify,
	openapi::{
		OpenApi,
		security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme},
	},
};

pub(super) struct SecurityAddon;

impl Modify for SecurityAddon
{
	fn modify(&self, openapi: &mut OpenApi)
	{
		let api_key_scheme =
			SecurityScheme::Http(Http::builder().scheme(HttpAuthScheme::Bearer).build());

		let session_auth_scheme =
			SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(SessionId::COOKIE_NAME)));

		let components = openapi.components.get_or_insert_default();
		components.add_security_scheme("api_key", api_key_scheme);
		components.add_security_scheme("session_auth", session_auth_scheme);
	}
}
