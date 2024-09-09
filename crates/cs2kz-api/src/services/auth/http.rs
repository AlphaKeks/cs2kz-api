//! This module contains the HTTP handlers for the `/auth` endpoint.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, State};
use axum::response::Redirect;
use axum::{routing, Router};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use http_body_util::BodyExt;
use problem_details::AsProblemDetails;
use serde::Deserialize;
use steam_openid::VerifyCallbackPayloadError;
use tower::{Service, ServiceBuilder, ServiceExt};
use tower_sessions::{CookieOptions, SessionID as _, SessionManagerLayer, Strict};
use url::Url;

use super::AuthService;
use crate::auth::{Session, SessionID};
use crate::extract::{Extension, Query};
use crate::middleware::infallible::InfallibleLayer;

const STEAM_INFO_COOKIE_NAME: &str = "kz-player";

/// Returns a router for the `/auth` endpoint.
pub fn router(auth_service: AuthService, cookie_options: Arc<CookieOptions>) -> Router
{
	let session_auth = SessionManagerLayer::new(
		Strict::RequireAuthentication,
		cookie_options,
		auth_service.session_store.clone(),
	);

	let auth = ServiceBuilder::new()
		.layer(InfallibleLayer::new())
		.map_err(crate::auth::error::SessionManagerErrorWithoutAuth::from)
		.layer(session_auth);

	Router::new()
		.route("/login", routing::get(login))
		.route("/logout", routing::get(logout).route_layer(auth))
		.route("/callback", routing::get(callback))
		.with_state(auth_service)
}

#[derive(Debug, Deserialize)]
struct LoginQuery
{
	redirect_to: Url,
}

async fn login(
	State(auth_service): State<AuthService>,
	Query(LoginQuery { redirect_to }): Query<LoginQuery>,
) -> Redirect
{
	let url =
		steam_openid::LoginForm::new(Arc::unwrap_or_clone(auth_service.public_url), "/callback")
			.redirect_url(&redirect_to)
			.expect("url should be serializable");

	Redirect::to(url.as_str())
}

#[derive(Debug, Deserialize)]
struct LogoutQuery
{
	#[serde(default)]
	invalidate_all_sessions: bool,
}

async fn logout(
	State(auth_service): State<AuthService>,
	Extension(session): Extension<Session>,
	Query(LogoutQuery {
		invalidate_all_sessions,
	}): Query<LogoutQuery>,
	cookies: CookieJar,
) -> crate::http::Result<CookieJar>
{
	session.invalidate();

	if invalidate_all_sessions {
		auth_service
			.session_store
			.invalidate_all_sessions(session.data().user_id())
			.await
			.map_err(|error| error.as_problem_details())?;
	}

	let steam_info_cookie = Cookie::build((STEAM_INFO_COOKIE_NAME, ""))
		.domain(auth_service.cookie_domain.to_string())
		.path("/")
		.secure(cfg!(feature = "production"))
		.http_only(false);

	Ok(cookies.remove(steam_info_cookie))
}

async fn callback(
	State(auth_service): State<AuthService>,
	ConnectInfo(peer_addr): ConnectInfo<SocketAddr>,
	Query(mut payload): Query<steam_openid::CallbackPayload>,
	cookies: CookieJar,
) -> crate::http::Result<(CookieJar, Redirect)>
{
	let http_service = tower::service_fn(|req: http::Request<reqwest::Body>| async {
		let req = reqwest::Request::try_from(req)?;
		let (parts, body) = (&auth_service.http_client)
			.map_response(http::Response::from)
			.call(req)
			.await?
			.into_parts();

		let body = body.collect().await?.to_bytes();

		Ok::<_, reqwest::Error>(http::Response::from_parts(parts, body))
	});

	match payload.verify(&auth_service.public_url, http_service).await {
		Ok(()) => {}
		Err(VerifyCallbackPayloadError::HttpClient(error)) => {}
		Err(VerifyCallbackPayloadError::ResponseBodyNotUtf8(error)) => {}
		Err(VerifyCallbackPayloadError::InvalidPayload) => {}
	}

	let user_id = payload
		.steam_id()
		.expect("payload should be valid")
		.try_into()
		.expect("should be valid SteamID");

	let user = auth_service
		.steam_service
		.get_user(user_id)
		.await
		.map_err(|error| error.as_problem_details())?;

	let session_id = auth_service
		.session_store
		.login(user_id, &user.username, peer_addr.ip().into())
		.await
		.map_err(|error| error.as_problem_details())?;

	let session_cookie = Cookie::build((SessionID::cookie_name(), session_id.to_string()))
		.domain(auth_service.cookie_domain.to_string())
		.path("/")
		.secure(cfg!(feature = "production"))
		.http_only(true)
		.same_site(SameSite::Lax)
		.build();

	let user_json = serde_json::to_string(&user).expect("valid json");
	let steam_info_cookie = Cookie::build((STEAM_INFO_COOKIE_NAME, user_json))
		.domain(auth_service.cookie_domain.to_string())
		.path("/")
		.secure(cfg!(feature = "production"))
		.http_only(false)
		.same_site(SameSite::Lax)
		.build();

	let cookies = cookies.add(session_cookie).add(steam_info_cookie);
	let redirect = Redirect::to(&payload.userdata);

	Ok((cookies, redirect))
}
