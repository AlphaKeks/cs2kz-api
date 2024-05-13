//! Steam Users.

use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use url::Url;
use utoipa::ToSchema;

use crate::http::HandlerError;
use crate::Config;

/// Steam WebAPI URL for fetching information about players.
const API_URL: &str = "http://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

/// Information about a Steam user.
///
/// This will be serialized as JSON and put into a cookie so frontends can use it.
#[derive(Debug, Serialize, ToSchema)]
pub struct User {
	/// The user's SteamID.
	pub steam_id: SteamID,

	/// Also the user's SteamID, but encoded as a stringified 64-bit integer, because
	/// JavaScript.
	pub steam_id64: String,

	/// The user's username.
	pub username: String,

	/// The user's "real" name.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub realname: Option<String>,

	/// The user's country.
	#[serde(skip_serializing_if = "Option::is_none")]
	pub country: Option<String>,

	/// URL to the user's profile.
	pub profile_url: Url,

	/// URL to the user's avatar.
	pub avatar_url: Url,
}

impl User {
	/// Fetch a user from Steam's API.
	///
	/// # Panics
	///
	/// This function will panic if it has a bug.
	pub async fn fetch(
		steam_id: SteamID,
		http_client: &reqwest::Client,
		config: &Config,
	) -> Result<Self, FetchSteamUserError> {
		let url = Url::parse_with_params(API_URL, [
			("key", config.steam_api_key.clone()),
			("steamids", steam_id.as_u64().to_string()),
		])
		.expect("valid url");

		let response = http_client.get(url).send().await?;

		if response.status().is_client_error() {
			return Err(FetchSteamUserError::InvalidSteamID);
		}

		if response.status().is_server_error() {
			let message = response.text().await?;

			return Err(FetchSteamUserError::SteamAPI(message));
		}

		let user = response.json::<Self>().await?;

		Ok(user)
	}
}

/// The different types of errors that can occur when fetching users from Steam's API.
#[derive(Debug, Error)]
pub enum FetchSteamUserError {
	/// Something went wrong during the HTTP request.
	#[error("failed to fetch user: {0}")]
	Http(#[from] reqwest::Error),

	/// The SteamID provided was invalid.
	#[error("invalid SteamID")]
	InvalidSteamID,

	/// Steam did not respond successfully.
	#[error("Steam API error: {0}")]
	SteamAPI(String),
}

impl From<FetchSteamUserError> for HandlerError {
	fn from(error: FetchSteamUserError) -> Self {
		let message = error.to_string();

		match error {
			FetchSteamUserError::Http(error) => Self::internal_server_error()
				.with_message(message)
				.with_source(error),
			FetchSteamUserError::InvalidSteamID => Self::unknown("user"),
			FetchSteamUserError::SteamAPI(_) => Self::bad_gateway().with_message(message),
		}
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
