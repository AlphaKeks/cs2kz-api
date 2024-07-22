//! HTTP handlers for this service.

use std::net::SocketAddr;

use axum::extract::{ConnectInfo, State};
use axum::response::Redirect;
use axum::{routing, Router};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::{CookieJar, Query};
use serde::Deserialize;
use time::OffsetDateTime;
use tower::ServiceBuilder;

use super::session::SessionManagerLayer;
use super::{session, AuthService, LoginRequest, LogoutRequest, Session};
use crate::middleware;
use crate::runtime::Result;
use crate::services::steam;

impl From<AuthService> for Router
{
	fn from(svc: AuthService) -> Self
	{
		let auth = ServiceBuilder::new()
			.layer(middleware::InfallibleLayer)
			.layer(SessionManagerLayer::new(svc.clone()));

		Router::new()
			.route("/login", routing::get(login))
			.route("/logout", routing::get(logout).route_layer(auth))
			.route("/callback", routing::get(callback))
			.with_state(svc)
	}
}

/// Login with Steam.
async fn login(State(svc): State<AuthService>, Query(req): Query<LoginRequest>) -> Redirect
{
	Redirect::to(svc.login_url(req).openid_url.as_str())
}

/// Query parameters for the `logout` handler.
#[derive(Debug, Deserialize)]
struct LogoutQuery
{
	/// Whether to invalidate all previous sessions, rather than just the
	/// current one.
	invalidate_all_sessions: bool,
}

/// Logout.
async fn logout(
	State(svc): State<AuthService>,
	Query(req): Query<LogoutQuery>,
	cookies: CookieJar,
	session: Session,
) -> Result<CookieJar>
{
	svc.logout(LogoutRequest { invalidate_all_sessions: req.invalidate_all_sessions, session })
		.await?;

	let session_cookie = Cookie::build((session::COOKIE_NAME, ""))
		.domain(svc.api_config.cookie_domain().to_owned())
		.path("/")
		.secure(cfg!(feature = "production"))
		.http_only(true)
		.expires(OffsetDateTime::now_utc())
		.build();

	Ok(cookies.add(session_cookie))
}

/// Hit by Steam after a successful login.
async fn callback(
	State(svc): State<AuthService>,
	ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
	openid_payload: steam::OpenIDPayload,
	cookies: CookieJar,
) -> Result<CookieJar>
{
	let user = svc.steam_svc.fetch_user(openid_payload.steam_id()).await?;
	let session_cookie = svc
		.login(user.steam_id, user.username, req_addr.ip())
		.await?
		.into_cookie(&svc.api_config);

	Ok(cookies.add(session_cookie))
}
