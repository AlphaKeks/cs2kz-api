//! A service for managing user authentication.
//!
//! Users authenticate with Steami via OpenID and authorize with a custom
//! permission system. [`AuthService::login_url()`] will produce a URL that the
//! user can visit to login with Steam. Steam will redirect them back to us
//! after a successful login (see the `callback()` function in the `http`
//! module). When this happens, [`AuthService::login()`] will create a session
//! in the database for the user.

#![allow(clippy::clone_on_ref_ptr)] // FIXME: remove when axum 0.8 releases

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::FromRef;
use cs2kz::SteamID;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{MySql, Pool};
use time::OffsetDateTime;

use self::session::User;
use crate::runtime;
use crate::services::SteamService;
use crate::util::AddrExt;

mod error;
pub use error::{Error, Result, SetupError};

mod models;
pub use models::{LoginRequest, LoginResponse, LogoutRequest};

pub mod session;
pub use session::{Session, SessionID};

pub mod jwt;
pub use jwt::Jwt;

mod server_info;
pub use server_info::ServerInfo;

mod http;

/// A service for managing user authentication.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct AuthService
{
	api_config: runtime::Config,
	database: Pool<MySql>,
	http_client: reqwest::Client,
	jwt_state: Arc<JwtState>,
	steam_svc: SteamService,
}

impl AuthService
{
	/// Create a new [`AuthService`].
	pub fn new(
		api_config: runtime::Config,
		database: Pool<MySql>,
		http_client: reqwest::Client,
		steam_svc: SteamService,
	) -> Result<Self, SetupError>
	{
		let jwt_state = Arc::new(JwtState {
			header: jsonwebtoken::Header::default(),
			encoding_key: jsonwebtoken::EncodingKey::from_base64_secret(api_config.jwt_secret())?,
			decoding_key: jsonwebtoken::DecodingKey::from_base64_secret(api_config.jwt_secret())?,
			validation: jsonwebtoken::Validation::default(),
		});

		Ok(Self { api_config, database, http_client, jwt_state, steam_svc })
	}

	/// Generates a URL that allows a user to login with Steam.
	pub fn login_url(&self, req: LoginRequest) -> LoginResponse
	{
		LoginResponse {
			openid_url: self
				.steam_svc
				.openid_login_form()
				.redirect_to(&req.redirect_to),
		}
	}

	/// Invalidates a user's login session(s).
	pub async fn logout(&self, req: LogoutRequest) -> Result<()>
	{
		sqlx::query! {
			r"
			UPDATE
			  LoginSessions
			SET
			  expires_on = NOW()
			WHERE
			  player_id = ?
			  AND expires_on > NOW()
			  AND (
			    id = ?
			    OR ?
			  )
			",
			req.session.user().steam_id(),
			req.session.id(),
			req.invalidate_all_sessions,
		}
		.execute(&self.database)
		.await?;

		tracing::trace! {
			session.id = %req.session.id(),
			session.user.id = %req.session.user().steam_id(),
			all = %req.invalidate_all_sessions,
			"invalidated session(s) for user",
		};

		Ok(())
	}

	/// Creates a new session for the given user.
	#[tracing::instrument(skip_all, fields(
		user.id = %user_id,
		user.name = %user_name,
		user.ip = %user_ip,
		session.id = tracing::field::Empty,
		session.expires_on = tracing::field::Empty,
	))]
	async fn login(&self, user_id: SteamID, user_name: String, user_ip: IpAddr) -> Result<Session>
	{
		let session_id = SessionID::new();
		let expires_on = generate_session_expiration_date();

		tracing::Span::current()
			.record("session.id", format_args!("{session_id}"))
			.record("session.expires_on", format_args!("{expires_on}"));

		tracing::debug!("creating new session");

		let mut txn = self.database.begin().await?;

		if sqlx::query! {
			r"
			INSERT INTO
			  Players (id, name, ip_address)
			VALUES
			  (?, ?, ?)
			",
			user_id,
			user_name,
			user_ip.to_v6(),
		}
		.execute(txn.as_mut())
		.await
		.is_ok()
		{
			tracing::debug!("user did not exist; created new entry");
		}

		sqlx::query! {
			r"
			INSERT INTO
			  LoginSessions (id, player_id, expires_on)
			VALUES
			  (?, ?, ?)
			",
			session_id,
			user_id,
			expires_on,
		}
		.execute(txn.as_mut())
		.await?;

		let user_permissions = sqlx::query! {
			r"
			SELECT
			  permissions `permissions: session::user::Permissions`
			FROM
			  Players
			WHERE
			  id = ?
			",
			user_id,
		}
		.fetch_one(txn.as_mut())
		.await
		.map(|user| user.permissions)?;

		txn.commit().await?;

		tracing::debug!("created session");

		Ok(Session::new(session_id, User::new(user_id, user_permissions)))
	}

	/// Encode a JWT into a string.
	pub fn encode_jwt<T>(&self, jwt: &Jwt<T>) -> Result<String>
	where
		T: Serialize,
	{
		jsonwebtoken::encode(&self.jwt_state.header, jwt.payload(), &self.jwt_state.encoding_key)
			.map_err(Error::EncodeJwt)
	}

	/// Decode a string as a JWT.
	pub fn decode_jwt<T>(&self, jwt: &str) -> Result<Jwt<T>>
	where
		T: DeserializeOwned,
	{
		jsonwebtoken::decode(jwt, &self.jwt_state.decoding_key, &self.jwt_state.validation)
			.map(|data| data.claims)
			.map_err(Error::DecodeJwt)
	}
}

/// State for encoding/decoding JWTs.
#[allow(clippy::missing_docs_in_private_items)]
struct JwtState
{
	header: jsonwebtoken::Header,
	encoding_key: jsonwebtoken::EncodingKey,
	decoding_key: jsonwebtoken::DecodingKey,
	validation: jsonwebtoken::Validation,
}

/// Generates a new expiration date for any given session.
fn generate_session_expiration_date() -> OffsetDateTime
{
	OffsetDateTime::now_utc() + (time::Duration::WEEK * 2)
}
