//! Global application state.
//!
//! When the API starts up, an instance of [`State`] will be created and leaked on the heap, so a
//! `'static` reference to it can be passed around easily.

use std::convert::Infallible;

use anyhow::Context;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use derive_more::Debug;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{MySql, Transaction};

use crate::authentication::Jwt;
use crate::Config;

/// The API's global state.
///
/// This is created once on startup, and then leaked on the heap.
/// A `'static` reference to it can then be passed around easily.
#[derive(Debug)]
pub struct State {
	/// The API's configuration.
	pub config: Config,

	/// A databse pool.
	#[debug(skip)]
	pub database: sqlx::Pool<MySql>,

	/// An HTTP client.
	#[debug(skip)]
	pub http_client: reqwest::Client,

	/// JWT header for encoding.
	#[debug(skip)]
	jwt_header: jwt::Header,

	/// JWT encoding secret key.
	#[debug(skip)]
	jwt_encoding_key: jwt::EncodingKey,

	/// JWT decoding secret key.
	#[debug(skip)]
	jwt_decoding_key: jwt::DecodingKey,

	/// JWT validations for decoding.
	#[debug(skip)]
	jwt_validation: jwt::Validation,
}

impl State {
	/// Creates a new [`State`] object.
	pub fn new(config: Config, database: sqlx::Pool<MySql>) -> anyhow::Result<Self> {
		let http_client = reqwest::Client::builder()
			.user_agent(concat!("CS2KZ API/", env!("CARGO_PKG_VERSION")))
			.build()
			.context("create http client")?;

		let jwt_header = jwt::Header::default();
		let jwt_encoding_key = jwt::EncodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_decoding_key = jwt::DecodingKey::from_base64_secret(&config.jwt_secret)?;
		let jwt_validation = jwt::Validation::default();

		Ok(Self {
			config,
			database,
			http_client,
			jwt_header,
			jwt_encoding_key,
			jwt_decoding_key,
			jwt_validation,
		})
	}

	/// Begins a new database transaction.
	///
	/// The returned [`Transaction`] object will automatically call [`Transaction::rollback()`]
	/// when it gets dropped, unless [`Transaction::commit()`] or [`Transaction::rollback()`]
	/// have been called manually.
	pub async fn begin_transaction(&self) -> sqlx::Result<Transaction<'_, MySql>> {
		self.database.begin().await
	}

	/// Encodes the given `value` as a JWT.
	pub fn encode_jwt<T>(&self, value: Jwt<&T>) -> jwt::errors::Result<String>
	where
		T: Serialize,
	{
		jwt::encode(&self.jwt_header, &value, &self.jwt_encoding_key)
	}

	/// Decode the claims of the given `jwt`.
	pub fn decode_jwt<S, T>(&self, jwt: S) -> jwt::errors::Result<Jwt<T>>
	where
		S: AsRef<str>,
		T: DeserializeOwned,
	{
		jwt::decode(jwt.as_ref(), &self.jwt_decoding_key, &self.jwt_validation)
			.map(|token| token.claims)
	}
}

#[async_trait]
impl FromRequestParts<&'static State> for &'static State {
	type Rejection = Infallible;

	async fn from_request_parts(
		_req: &mut request::Parts,
		state: &&'static State,
	) -> Result<Self, Self::Rejection> {
		Ok(state)
	}
}
