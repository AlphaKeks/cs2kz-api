use {
	headers::HeaderMapExt,
	http::{HeaderValue, Method, header, request},
	std::sync::Arc,
	tower_http::cors::{AllowCredentials, AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
};

pub(crate) fn layer(allowed_origins: impl IntoIterator<Item = HeaderValue>) -> CorsLayer
{
	let allowed_origins = Arc::<[HeaderValue]>::from_iter(allowed_origins);

	CorsLayer::default()
		.allow_credentials(AllowCredentials::predicate({
			let allowed_origins = Arc::clone(&allowed_origins);
			move |origin, request| -> bool {
				self::allow_credentials(&allowed_origins[..], origin, request)
			}
		}))
		.allow_headers(AllowHeaders::mirror_request())
		.allow_methods(AllowMethods::mirror_request())
		.allow_origin(AllowOrigin::predicate(move |origin, _request| -> bool {
			allowed_origins.contains(origin)
		}))
		.expose_headers([header::COOKIE])
}

fn allow_credentials(
	allowed_origins: &[HeaderValue],
	origin: &HeaderValue,
	request: &request::Parts,
) -> bool
{
	if !allowed_origins.contains(origin) {
		return false;
	}

	macro sensitive_methods() {
		Method::POST | Method::PUT | Method::PATCH | Method::DELETE
	}

	match request.method {
		sensitive_methods!() => true,
		Method::GET => request.uri.path().starts_with("/auth"),
		Method::OPTIONS => request
			.headers
			.typed_get::<headers::AccessControlRequestMethod>()
			.map(Method::from)
			.is_some_and(|method| matches!(method, sensitive_methods!())),
		_ => false,
	}
}
