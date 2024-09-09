//! This module contains the [`SteamService`].

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;

use cs2kz::SteamID;
use derive_more::{Constructor, Debug};
use serde::Serialize;
use tokio::process::Command;

mod errors;
pub use errors::{DownloadMapError, GetMapNameError, GetUserError};

mod user;
pub use user::User;

mod map_name;
use map_name::GetMapNameResponse;

mod map_file;
pub use map_file::{MapFileHash, MapHandle};

make_id! {
	/// A Steam Workshop ID.
	pub struct WorkshopID(u32);
}

/// The Steam service.
#[derive(Debug, Constructor, Clone)]
pub struct SteamService
{
	/// Steam WebAPI key.
	api_key: Arc<str>,

	/// HTTP client so we can make requests to Steam's API.
	#[debug("reqwest::Client")]
	http_client: reqwest::Client,

	/// Directory to use for storing downloaded workshop assets (e.g. map files).
	workshop_asset_dir: Arc<Path>,

	/// Path to a `DepotDownloader` executable.
	depot_downloader: Arc<Path>,
}

impl SteamService
{
	/// Fetches a user from the Steam API.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_user(&self, user_id: SteamID) -> Result<User, GetUserError>
	{
		const URL: &str = "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002";

		#[derive(Constructor, Serialize)]
		struct QueryParams<'a>
		{
			#[serde(rename = "steamids", serialize_with = "SteamID::serialize_u64")]
			user_id: SteamID,
			key: &'a str,
		}

		let response = self
			.http_client
			.get(URL)
			.query(&QueryParams::new(user_id, &self.api_key))
			.send()
			.await?;

		if let Err(error) = response.error_for_status_ref() {
			let response_body = response.text().await.ok();
			error!(?error, ?response_body, "failed to fetch user from Steam");
			return Err(error.into());
		}

		response.json::<User>().await.map_err(Into::into)
	}

	/// Fetches a map's name from the Steam API.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn get_map_name(&self, workshop_id: WorkshopID) -> Result<String, GetMapNameError>
	{
		const URL: &str =
			"https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

		#[derive(Constructor, Serialize)]
		struct QueryParams
		{
			#[serde(rename = "publishedfileids[0]")]
			workshop_id: WorkshopID,
			itemcount: u8,
		}

		let response = self
			.http_client
			.post(URL)
			.form(&QueryParams::new(workshop_id, 1))
			.send()
			.await?;

		if let Err(error) = response.error_for_status_ref() {
			let response_body = response.text().await.ok();
			error!(?error, ?response_body, "failed to fetch map from Steam");
			return Err(error.into());
		}

		response
			.json::<GetMapNameResponse>()
			.await
			.map(|response| response.name)
			.map_err(Into::into)
	}

	/// Downloads a workshop map.
	///
	/// The returned handle can then be used to interact with the file.
	#[instrument(err(Debug, level = "debug"))]
	pub async fn download_map(&self, workshop_id: WorkshopID)
		-> Result<MapHandle, DownloadMapError>
	{
		trace!("invoking DepotDownloader");

		let mut cmd = Command::new(self.depot_downloader.as_os_str());

		cmd.args(["-app", "730", "-pubfile"])
			.arg(workshop_id.to_string())
			.arg("-dir")
			.arg(self.workshop_asset_dir.as_os_str())
			.stdout(Stdio::piped())
			.stderr(Stdio::piped());

		// TODO: stream output from stdout/stderr to the caller via AsyncRead
		let mut child = cmd.spawn().map_err(DownloadMapError::SpawnChild)?;

		// just in case
		drop(child.stdin.take());

		match child.wait().await {
			Ok(exit_code) if exit_code.success() => {
				let filepath = self.workshop_asset_dir.join(format!("{workshop_id}.vpk"));
				let handle = MapHandle::new(&filepath).await.map_err(|error| {
					error!(?error, ?filepath, "failed to open map file");
					DownloadMapError::OpenMapFile(error)
				})?;

				Ok(handle)
			}
			Ok(exit_code) => {
				error!(?exit_code, "DepotDownloader did not exit successfully");
				Err(DownloadMapError::NonZeroExitCode)
			}
			Err(error) => {
				error!(?error, "failed to wait for DepotDownloader");
				Err(DownloadMapError::WaitForChild)
			}
		}
	}
}
