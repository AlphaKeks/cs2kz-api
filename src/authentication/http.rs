//! HTTP handlers for the `/auth` endpoint.

use std::net::SocketAddr;

use authentication::Session;
use axum::extract::{ConnectInfo, Query, State};
use axum::http::{Method, StatusCode};
use axum::response::Redirect;
use axum::{routing, Router};
use axum_extra::extract::CookieJar;

use super::{AuthService, LoginRequest, LogoutRequest};
use crate::middleware::cors;
use crate::openapi::responses;
use crate::{authentication, steam, Result};

impl From<AuthService> for Router
{
	fn from(state: AuthService) -> Self
	{
		let logout = Router::new()
			.route("/logout", routing::get(logout))
			.route_layer(cors::dashboard([Method::GET]))
			.with_state(state.clone());

		Router::new()
			.route("/login", routing::get(login))
			.route("/callback", routing::get(callback))
			.route_layer(cors::permissive())
			.with_state(state.clone())
			.merge(logout)
	}
}

/// Login with Steam.
///
/// This will redirect the user to Steam, where they can login. A session for
/// them will be created and they're redirected back to `redirect_to`.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/login",
  tag = "Auth",
  params(LoginRequest),
  responses(//
    responses::SeeOther,
    responses::BadRequest,
  ),
)]
pub async fn login(State(state): State<AuthService>, Query(login): Query<LoginRequest>)
-> Redirect
{
	let steam_url = state.login(login).await;

	Redirect::to(steam_url.as_str())
}

/// Logout again.
///
/// This will invalidate your current session, and potentially every other
/// session as well (if `invalidate_all_sessions=true` is specified).
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/logout",
  tag = "Auth",
  security(("Browser Session" = [])),
  params(LogoutRequest),
  responses(
    responses::SeeOther,
    responses::BadRequest,
    responses::Unauthorized,
  ),
)]
pub async fn logout(
	mut session: Session,
	State(state): State<AuthService>,
	Query(logout): Query<LogoutRequest>,
) -> Result<(Session, StatusCode)>
{
	state.logout(&mut session, logout).await?;

	Ok((session, StatusCode::OK))
}

/// The endpoint hit by Steam after a successful login.
///
/// This should not be used directly, and trying to do so will lead to errors.
#[tracing::instrument(skip(state))]
#[utoipa::path(
  get,
  path = "/auth/callback",
  tag = "Auth",
  params(authentication::steam::LoginResponse),
  responses(
    responses::Ok<()>,
    responses::NoContent,
    responses::BadRequest,
  ),
)]
pub async fn callback(
	login: authentication::steam::LoginResponse,
	user: steam::User,
	cookies: CookieJar,
	State(state): State<AuthService>,
	ConnectInfo(req_addr): ConnectInfo<SocketAddr>,
) -> Result<(CookieJar, Redirect)>
{
	let session = state.create_session(&user, req_addr.ip()).await?;
	let user_cookie = user.to_cookie(&state.api_config);
	let cookies = cookies.add(session).add(user_cookie);
	let redirect = Redirect::to(login.redirect_to.as_str());

	tracing::debug!("user logged in");

	Ok((cookies, redirect))
}
