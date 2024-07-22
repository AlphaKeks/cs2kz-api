use std::io;

use tap::{Pipe, Tap, TryConv};
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;

use crate::{runtime, util};

util::make_id! {
	/// An ID uniquely identifying a Steam Workshop item.
	WorkshopID as u32
}

/// A handle to a downloaded workshop map.
#[derive(Debug)]
#[must_use = "this type opens a file and allocates a buffer"]
pub struct MapFile
{
	/// OS handle to the open file descriptor.
	handle: File,
}

impl MapFile
{
	/// Downloads a map using [DepotDownloader] and returns a handle to the
	/// acquired file.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	pub(super) async fn download(id: WorkshopID, api_config: &runtime::Config) -> io::Result<Self>
	{
		#[cfg(feature = "production")]
		let out_dir = api_config.workshop_artifacts_path();

		#[cfg(not(feature = "production"))]
		let out_dir = api_config
			.workshop_artifacts_path()
			.ok_or_else(|| io::Error::other("missing workshop artifacts directory"))?;

		#[cfg(feature = "production")]
		let depot_downloader = api_config.depot_downloader_path();

		#[cfg(not(feature = "production"))]
		let depot_downloader = api_config
			.depot_downloader_path()
			.ok_or_else(|| io::Error::other("missing depot downloader path"))?;

		let result = Command::new(depot_downloader)
			.args(["-app", "730", "-pubfile"])
			.arg(id.to_string())
			.arg("-dir")
			.arg(out_dir)
			.spawn()?
			.wait_with_output()
			.await?;

		let mut stdout = tokio::io::stdout();
		let mut stderr = tokio::io::stderr();

		if let Err(error) = tokio::try_join!(stdout.flush(), stderr.flush()) {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				%error,
				"failed to flush stdout/stderr",
			};
		}

		if !result.status.success() {
			return Err(io::Error::other("DepotDownloader did not complete successfully"));
		}

		let out_file_path = out_dir.join(format!("{id}.vpk"));
		let file = File::open(&out_file_path).await.inspect_err(|err| {
			tracing::error! {
				target: "cs2kz_api::audit_log",
				%err,
				path = ?out_file_path,
				"failed to open map file",
			};
		})?;

		Ok(Self { handle: file })
	}

	/// Computes the crc32 checksum of this file.
	pub async fn checksum(mut self) -> io::Result<u32>
	{
		let mut buf = self
			.handle
			.metadata()
			.await?
			.pipe(|metadata| metadata.len())
			.try_conv::<usize>()
			.map(Vec::with_capacity)
			.expect("64-bit platform");

		self.handle.read_to_end(&mut buf).await?;

		Ok(crc32fast::hash(&buf))
	}
}
