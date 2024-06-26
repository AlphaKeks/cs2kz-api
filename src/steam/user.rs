//! Steam user information.

use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request;
use axum_extra::extract::cookie::Cookie;
use cs2kz::SteamID;
use derive_more::Debug;
use serde::{Deserialize, Deserializer, Serialize};
use url::Url;
use utoipa::ToSchema;

use crate::{Error, Result, State};

/// Steam Web API URL for fetching user information.
const API_URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

/// HTTP cookie name for storing a serialized [`User`].
const COOKIE_NAME: &str = "kz-player";

/// A Steam user.
#[derive(Debug, Serialize, ToSchema)]
pub struct User {
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// The user's SteamID in its stringified 64-bit format.
	pub steam_id64: String,

	/// The user's username.
	pub username: String,

	/// The user's realname.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub realname: Option<String>,

	/// The user's country.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,

	/// URL to the user's Steam profile.
	pub profile_url: Url,

	/// URL to the user's Steam avatar.
	pub avatar_url: Url,
}

impl User {
	/// Fetches a user from Steam's API.
	#[tracing::instrument(level = "debug", skip(http_client, api_config))]
	pub async fn fetch(
		steam_id: SteamID,
		http_client: &reqwest::Client,
		api_config: &crate::Config,
	) -> Result<Self> {
		let url = Url::parse_with_params(API_URL, [
			("key", api_config.steam_api_key.clone()),
			("steamids", steam_id.as_u64().to_string()),
		])
		.map_err(|err| Error::logic("failed to parse url").context(err))?;

		let response = http_client.get(url).send().await?;

		if let Err(error) = response.error_for_status_ref() {
			let error = Error::external_api_call(error);
			let response_body = response.text().await.ok();

			tracing::error!(?error, ?response_body, "failed to fetch steam user");

			return Err(error.context(format!("response body: {response_body:?}")));
		}

		let user = response.json::<Self>().await?;

		Ok(user)
	}

	/// Generates a fake user for use in tests.
	#[cfg(test)]
	pub fn invalid(steam_id: SteamID) -> Self {
		let url = Url::parse("https://cs2kz.org").unwrap();

		Self {
			steam_id,
			steam_id64: steam_id.as_u64().to_string(),
			username: String::from("schnose"),
			realname: None,
			country: None,
			profile_url: url.clone(),
			avatar_url: url,
		}
	}

	/// Serializes this user into an HTTP cookie.
	pub fn to_cookie(&self, api_config: &crate::Config) -> Cookie<'static> {
		let json = serde_json::to_string(self).expect("this is valid json");

		Cookie::build((COOKIE_NAME, json))
			.domain(api_config.cookie_domain.clone())
			.path("/")
			.secure(cfg!(feature = "production"))
			.http_only(false)
			.permanent()
			.build()
	}
}

impl<'de> Deserialize<'de> for User {
	#[allow(clippy::missing_docs_in_private_items)]
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		#[derive(Deserialize)]
		struct Helper1 {
			response: Helper2,
		}

		#[derive(Deserialize)]
		struct Helper2 {
			players: [Helper3; 1],
		}

		#[derive(Deserialize)]
		struct Helper3 {
			steamid: SteamID,
			personaname: String,
			realname: Option<String>,
			loccountrycode: Option<String>,
			profileurl: Url,
			avatar: Url,
		}

		Helper1::deserialize(deserializer).map(|x| x.response).map(
			|Helper2 { players: [player] }| Self {
				steam_id: player.steamid,
				steam_id64: player.steamid.as_u64().to_string(),
				username: player.personaname,
				realname: player.realname,
				country: player.loccountrycode,
				profile_url: player.profileurl,
				avatar_url: player.avatar,
			},
		)
	}
}

#[async_trait]
impl FromRequestParts<State> for User {
	type Rejection = Error;

	#[tracing::instrument(
		level = "debug",
		name = "steam::user::from_request_parts",
		skip_all,
		fields(steam_id = tracing::field::Empty),
		err(level = "debug"),
	)]
	async fn from_request_parts(parts: &mut request::Parts, state: &State) -> Result<Self> {
		let steam_id = parts
			.extensions
			.get::<SteamID>()
			.copied()
			.expect("`SteamLoginResponse` extractor should have inserted this");

		tracing::Span::current().record("steam_id", format_args!("{steam_id}"));
		tracing::debug!("fetching user from steam");

		Self::fetch(steam_id, &state.http_client, &state.config).await
	}
}
