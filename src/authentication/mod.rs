//! Everything related to authentication.
//!
//! This module contains types, traits, and HTTP handlers related to
//! authentication. This includes JWT, sessions, and opaque API keys.

#![allow(clippy::clone_on_ref_ptr)] // TODO: remove when new axum version fixes

use std::net::IpAddr;
use std::sync::Arc;

use axum::extract::FromRef;
use sqlx::{MySql, Pool};
use url::Url;

use crate::authorization::AuthorizeSession;
use crate::Result;

mod jwt;
pub use jwt::{Jwt, JwtState};

mod server;
pub use server::Server;

pub mod session;
pub use session::Session;

pub mod api_key;
pub use api_key::ApiKey;

mod user;
pub use user::User;

pub mod steam;

mod models;
pub use models::{LoginRequest, LogoutRequest};

pub mod http;

/// A service for dealing with authentication.
#[derive(Clone, FromRef)]
#[allow(missing_debug_implementations, clippy::missing_docs_in_private_items)]
pub struct AuthService
{
	database: Pool<MySql>,
	api_config: Arc<crate::Config>,
	http_client: reqwest::Client,
}

impl AuthService
{
	/// Creates a new [`AuthService`] instance.
	pub const fn new(
		database: Pool<MySql>,
		api_config: Arc<crate::Config>,
		http_client: reqwest::Client,
	) -> Self
	{
		Self { database, api_config, http_client }
	}

	/// Creates a Steam URL that a user can navigate to in order to login with
	/// Steam.
	pub async fn login(&self, login: LoginRequest) -> Url
	{
		steam::LoginForm::new(self.api_config.public_url.clone()).redirect_to(&login.redirect_to)
	}

	/// Invalidates one or more login session(s).
	pub async fn logout<A>(&self, session: &mut Session<A>, logout: LogoutRequest) -> Result<()>
	where
		A: AuthorizeSession,
	{
		let mut transaction = self.database.begin().await?;

		session
			.invalidate(logout.invalidate_all_sessions, &mut transaction)
			.await?;

		transaction.commit().await?;

		tracing::debug!("user logged out");

		Ok(())
	}

	/// Creates a new user session.
	pub async fn create_session(
		&self,
		user: &crate::steam::User,
		user_ip: IpAddr,
	) -> Result<Session>
	{
		let transaction = self.database.begin().await?;
		let session = Session::create(user, user_ip, &self.api_config, transaction).await?;

		Ok(session)
	}
}
