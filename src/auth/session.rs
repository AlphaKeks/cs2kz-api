use std::fmt::Write;
use std::sync::Arc;

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::{request, Uri};
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use sqlx::MySqlExecutor;
use tracing::{info, trace, warn};

use super::permissions::Permissions;
use super::Subdomain;
use crate::extractors::SessionToken;
use crate::{Error, Result, State};

#[derive(Debug, Clone)]
pub struct Session {
	pub steam_id: SteamID,
	pub permissions: Permissions,
	pub token: SessionToken,
}

impl Session {
	/// Creates a new session for a given player, on the given `subdomain` and `domain`.
	/// The cookie returned by this function can be used by the client for future requests.
	pub async fn create(
		steam_id: SteamID,
		subdomain: Option<Subdomain>,
		domain: impl Into<String>,
		secure: bool,
		executor: impl MySqlExecutor<'_>,
	) -> Result<Cookie<'static>> {
		let token = SessionToken::random();

		sqlx::query! {
			r#"
			INSERT INTO
			  WebSessions (subdomain, token, steam_id, expires_on)
			VALUES
			  (?, ?, ?, DATE_ADD(NOW(), INTERVAL 7 DAY))
			"#,
			subdomain,
			token,
			steam_id,
		}
		.execute(executor)
		.await?;

		let mut domain = domain.into();

		if let Some(subdomain) = subdomain {
			domain = format!("{subdomain}.{domain}");
		}

		info!(%steam_id, ?subdomain, %domain, ?token, "created session");

		let mut name = SessionToken::COOKIE_NAME.to_owned();

		if let Some(subdomain) = subdomain {
			write!(&mut name, "-{subdomain}").expect("this never fails");
		}

		let cookie = Cookie::build((name, token.0.to_string()))
			.secure(secure)
			.domain(domain)
			.build();

		Ok(cookie)
	}
}

#[async_trait]
impl FromRequestParts<Arc<State>> for Session {
	type Rejection = Error;

	async fn from_request_parts(parts: &mut request::Parts, state: &Arc<State>) -> Result<Self> {
		let uri = Uri::from_request_parts(parts, state)
			.await
			.map_err(|_| Error::Unauthorized)?;

		let subdomain = uri.host().and_then(|host| Some(host.split_once('.')?.0));

		#[rustfmt::skip]
		let max_permissions = match subdomain {
			subdomain if state.in_dev() => {
				warn!(?subdomain, "allowing subdomain due to dev mode");
				Permissions::NONE // TODO
			}

			None => Permissions::NONE,

			Some("dashboard") => Permissions::MAPS_VIEW
				| Permissions::MAPS_APPROVE
				| Permissions::MAPS_EDIT
				| Permissions::MAPS_DEGLOBAL
				| Permissions::SERVERS_APPROVE
				| Permissions::SERVERS_EDIT
				| Permissions::SERVERS_DEGLOBAL
				| Permissions::BANS_CREATE
				| Permissions::BANS_EDIT
				| Permissions::BANS_REMOVE,

			Some("forum" | "docs") => Permissions::NONE,

			subdomain => {
				trace!(?subdomain, "rejecting unknown subdomain");
				return Err(Error::Unauthorized);
			}
		};

		let token = SessionToken::from_request_parts(parts, state).await?;

		let user = sqlx::query! {
			r#"
			SELECT
			  p.steam_id `steam_id: SteamID`,
			  a.permissions,
			  s.subdomain
			FROM
			  WebSessions s
			  JOIN Players p ON p.steam_id = s.steam_id
			  LEFT JOIN Admins a ON a.steam_id = s.steam_id
			WHERE
			  s.token = ?
			  AND CURRENT_TIMESTAMP() < s.expires_on
			"#,
			token,
		}
		.fetch_optional(state.database())
		.await?
		.ok_or(Error::Unauthorized)?;

		if subdomain != user.subdomain.as_deref() {
			return Err(Error::Forbidden);
		}

		let permissions = max_permissions & user.permissions.map(Permissions).unwrap_or_default();

		Ok(Self { steam_id: user.steam_id, token, permissions })
	}
}