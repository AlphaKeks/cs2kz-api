//! Steam Workshop utilities.

use derive_more::Debug;
use futures::TryFutureExt;

use crate::http::HandlerError;
use crate::Config;

mod error;

#[doc(inline)]
pub use error::WorkshopError;

mod map_info;

#[doc(inline)]
pub use map_info::fetch_map_name;

mod map_file;

#[doc(inline)]
pub use map_file::MapFile;

/// URL for fetching map information from Steam's API.
const API_URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

#[cs2kz_api_macros::id]
pub struct WorkshopID(pub u32);

/// Fetch a map from the Workshop, download it, and compute its checksum.
pub async fn fetch_and_download_map(
	workshop_id: WorkshopID,
	http_client: &reqwest::Client,
	config: &Config,
) -> Result<(String, u32), HandlerError> {
	let (name, checksum) = tokio::try_join! {
		fetch_map_name(workshop_id, http_client).map_err(Into::into),
		MapFile::download(workshop_id, config).map_err(|err| {
			HandlerError::internal_server_error()
				.with_message("failed to download workshop map")
				.with_source(err)
		})
		.and_then(|map| async move {
			map.checksum().await.map_err(|err| {
				HandlerError::internal_server_error().with_message("failed to checksum workshop map").with_source(err)
			})
		}),
	}?;

	Ok((name, checksum))
}
