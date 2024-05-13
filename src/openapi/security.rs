//! OpenAPI security documentation.

use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::openapi::OpenApi;

use crate::authentication;

/// Shim for implementing [`utoipa::Modify`]
pub struct Security;

impl utoipa::Modify for Security {
	fn modify(&self, openapi: &mut OpenApi) {
		let components = openapi.components.get_or_insert_with(Default::default);
		let cs_server_jwt = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));
		let api_key = SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer));
		let sessions = SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(
			authentication::session::COOKIE_NAME,
		)));

		components.add_security_schemes_from_iter([
			("CS2 Server", cs_server_jwt),
			("API Key", api_key),
			("Browser Session", sessions),
		])
	}
}
