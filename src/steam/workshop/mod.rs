pub use self::id::WorkshopId;
use {
	crate::steam,
	futures_util::{StreamExt, TryFutureExt, stream},
	serde::{Deserialize, Serialize, Serializer, ser::SerializeMap},
	std::{error::Error, io, path::Path, process::Stdio, time::Duration},
	steam_id::SteamId,
	tokio::{process::Command, sync::Semaphore, task},
	tokio_util::{
		codec::{FramedRead, LinesCodec},
		time::FutureExt,
	},
	tracing::Instrument,
};

mod id;

const URL: &str = "https://api.steampowered.com/ISteamRemoteStorage/GetPublishedFileDetails/v1";

static DEPOT_DOWNLOADER_PERMITS: Semaphore = Semaphore::const_new(32);

#[derive(Debug)]
pub struct MapMetadata
{
	pub name: Box<str>,
	pub creator_id: SteamId,
}

#[instrument(skip(api_client), ret(level = "debug"), err(level = "debug"))]
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

#[instrument(ret(level = "debug"), err(level = "debug"))]
pub async fn download(
	id: WorkshopId,
	depot_downloader_path: &Path,
	out_dir: &Path,
) -> io::Result<Box<Path>>
{
	trace!(target: "cs2kz_api::depot_downloader", "acquiring permit");

	// TODO: timeout?
	let Ok(permit) = DEPOT_DOWNLOADER_PERMITS
		.acquire()
		.map_err(|err| panic!("static semaphore dropped? {err}"))
		.await;

	debug!(target: "cs2kz_api::depot_downloader", "spawning DepotDownloader process");

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
						debug!(target: "cs2kz_api::depot_downloader", source, "{line}");
					},
					Err(error) => {
						error!(
							error = &error as &dyn Error,
							"failed to read line from DepotDownloader's stdout",
						);
					},
				}
			}

			info!("DepotDownloader exited");
		}
		.in_current_span()
	});

	if !process.wait().await?.success() {
		let error = io::Error::other("DepotDownloader did not exit successfully");
		error!(error = &error as &dyn Error);
		return Err(error);
	}

	drop(permit);

	let timeout = Duration::from_secs(3);

	if let Err(_) = output_task.timeout(timeout).await {
		warn!(?timeout, "DepotDownloader output task did not exit within timeout");
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
