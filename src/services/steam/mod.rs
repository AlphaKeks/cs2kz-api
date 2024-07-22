//! A service for interacting with Steam.

use axum::extract::FromRef;
use cs2kz::SteamID;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value as JsonValue;
use url::Url;

use crate::runtime;

mod error;
pub use error::{Error, Result};

pub mod openid;
pub use openid::OpenIDPayload;

mod user;
pub use user::User;

pub mod workshop;
pub use workshop::WorkshopID;

mod http;

/// Steam Web API URL for fetching user information.
const USER_URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

/// Steam Web API URL for fetching map information.
const MAP_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

/// A service for interacting with Steam.
#[derive(Clone, FromRef)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct SteamService
{
	api_config: runtime::Config,
	http_client: reqwest::Client,
}

impl SteamService
{
	/// Create a new [`SteamService`].
	pub fn new(api_config: runtime::Config, http_client: reqwest::Client) -> Self
	{
		Self { api_config, http_client }
	}

	/// Build OpenID form parameters to send to Steam.
	pub fn openid_login_form(&self) -> openid::LoginForm
	{
		openid::LoginForm::new(self.api_config.public_url().clone())
	}

	/// Fetch information about a user.
	pub async fn fetch_user(&self, user_id: SteamID) -> Result<User>
	{
		let url = Url::parse_with_params(USER_URL, [
			("key", self.api_config.steam_api_key().to_owned()),
			("steamids", user_id.as_u64().to_string()),
		])
		.expect("valid url");

		let response = self.http_client.get(url).send().await?;

		if let Err(error) = response.error_for_status_ref() {
			let error = Error::Http(error);
			let response_body = response.text().await.ok();

			tracing::error!(?error, ?response_body, "failed to fetch steam user");

			return Err(error);
		}

		let user = response.json::<User>().await?;

		Ok(user)
	}

	/// Fetches a map's name from the workshop.
	pub async fn fetch_map_name(&self, workshop_id: WorkshopID) -> Result<String>
	{
		#[derive(Serialize)]
		#[allow(clippy::missing_docs_in_private_items)]
		struct Params
		{
			workshop_id: WorkshopID,
		}

		#[allow(clippy::missing_docs_in_private_items)]
		struct MapInfo
		{
			title: String,
		}

		impl<'de> Deserialize<'de> for MapInfo
		{
			#[allow(clippy::missing_docs_in_private_items)]
			fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
			where
				D: Deserializer<'de>,
			{
				#[derive(Serialize)]
				#[allow(clippy::missing_docs_in_private_items)]
				struct Params
				{
					workshop_id: WorkshopID,
				}

				#[derive(Deserialize)]
				struct Helper1
				{
					response: Helper2,
				}

				#[derive(Deserialize)]
				struct Helper2
				{
					publishedfiledetails: Vec<JsonValue>,
				}

				Helper1::deserialize(deserializer)
					.map(|x| x.response)
					.map(|mut x| x.publishedfiledetails.remove(0))
					.map(|mut json| json.get_mut("title").unwrap_or(&mut JsonValue::Null).take())
					.map(|json| json.as_str().map(ToOwned::to_owned))?
					.map(|title| Self { title })
					.ok_or_else(|| serde::de::Error::missing_field("title"))
			}
		}

		let response = self
			.http_client
			.post(MAP_URL)
			.form(&Params { workshop_id })
			.send()
			.await?;

		if !response.status().is_success() {
			return Err(Error::InvalidWorkshopID { workshop_id });
		}

		let name = response.json::<MapInfo>().await.map(|info| info.title)?;

		Ok(name)
	}

	/// Downloads a map from the workshop.
	pub async fn download_map(&self, workshop_id: WorkshopID) -> Result<workshop::MapFile>
	{
		workshop::MapFile::download(workshop_id, &self.api_config)
			.await
			.map_err(Error::DownloadWorkshopMap)
	}
}
