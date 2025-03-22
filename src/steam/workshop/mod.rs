mod id;

use std::{error::Error, io, path::Path, process::Stdio, time::Duration};

use futures_util::{StreamExt, stream};
use serde::{Deserialize, Serialize, Serializer, ser::SerializeMap};
use steam_id::SteamId;
use tokio::{process::Command, task};
use tokio_util::{
	codec::{FramedRead, LinesCodec},
	time::FutureExt,
};
use tracing::Instrument;

pub use self::id::WorkshopId;
use crate::steam;

const URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

#[derive(Debug)]
pub struct MapMetadata
{
	pub name: Box<str>,
	pub creator_id: SteamId,
}

#[tracing::instrument(skip(api_client), ret(level = "debug"), err(level = "debug"))]
pub async fn get_map_metadata(
	api_client: &steam::api::Client,
	id: WorkshopId,
) -> Result<Option<MapMetadata>, steam::ApiError>
{
	struct Form
	{
		id: WorkshopId,
	}

	impl Serialize for Form
	{
		fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
		{
			let mut serializer = serializer.serialize_map(Some(2))?;
			serializer.serialize_entry("itemcount", &1_u32)?;
			serializer.serialize_entry("publishedfileids[0]", &self.id)?;
			serializer.end()
		}
	}

	let request = api_client.as_ref().post(URL).form(&Form { id });

	let Response { mut publishedfiledetails } = steam::api::send_request(request).await?;

	Ok(if publishedfiledetails.is_empty() {
		None
	} else {
		let data = publishedfiledetails.remove(0);
		Some(MapMetadata { name: data.title, creator_id: data.creator })
	})
}

#[tracing::instrument(ret(level = "debug"), err(level = "debug"))]
pub async fn download(
	id: WorkshopId,
	depot_downloader_path: &Path,
	out_dir: &Path,
) -> io::Result<Box<Path>>
{
	tracing::debug!(
		target: "cs2kz_api::depot_downloader",
		exe_path = %depot_downloader_path.display(),
		out_dir = %out_dir.display(),
		"spawning DepotDownloader process",
	);

	let mut process = Command::new(depot_downloader_path)
		.args(["-app", "730", "-pubfile"])
		.arg(id.to_string())
		.arg("-dir")
		.arg(out_dir)
		.stdin(Stdio::null())
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.spawn()?;

	let stdout = process.stdout.take().unwrap_or_else(|| {
		panic!("took process stdout twice?");
	});

	let stderr = process.stderr.take().unwrap_or_else(|| {
		panic!("took process stderr twice?");
	});

	drop(process.stdin.take());

	let output_task = task::spawn({
		let stdout = FramedRead::new(stdout, LinesCodec::new()).map(|result| (result, "stdout"));
		let stderr = FramedRead::new(stderr, LinesCodec::new()).map(|result| (result, "stderr"));
		let mut output = stream::select(stdout, stderr);

		async move {
			while let Some((maybe_line, source)) = output.next().await {
				match maybe_line {
					Ok(line) => {
						tracing::debug!(target: "cs2kz_api::depot_downloader", source, "{line}");
					},
					Err(error) => {
						tracing::error!(
							error = &error as &dyn Error,
							"failed to read line from DepotDownloader's stdout",
						);
					},
				}
			}

			tracing::info!("DepotDownloader exited");
		}
		.in_current_span()
	});

	if !process.wait().await?.success() {
		let error = io::Error::other("DepotDownloader did not exit successfully");
		tracing::error!(error = &error as &dyn Error);
		return Err(error);
	}

	let timeout = Duration::from_secs(3);

	if let Err(_) = output_task.timeout(timeout).await {
		tracing::warn!(?timeout, "DepotDownloader output task did not exit within timeout");
	}

	Ok(out_dir.join(format!("{id}.vpk")).into_boxed_path())
}

#[derive(Debug, Deserialize)]
struct Response
{
	publishedfiledetails: Vec<PublishedFileDetails>,
}

#[derive(Debug, Deserialize)]
struct PublishedFileDetails
{
	title: Box<str>,
	creator: SteamId,
}
