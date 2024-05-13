//! This module contains functionality around downloading maps from the Steam Workshop.

use derive_more::Debug;
use thiserror::Error;
use tokio::fs::File;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::error;

use crate::steam::workshop::WorkshopID;
use crate::Config;

/// A handle to a downloaded Workshop map.
#[derive(Debug)]
pub struct MapFile {
	/// Handle to the file on disk.
	#[debug(skip)]
	file: File,
}

impl MapFile {
	/// Download this map from the workshop and return a handle to it.
	pub async fn download(
		workshop_id: WorkshopID,
		config: &Config,
	) -> Result<Self, DownloadWorkshopMapError> {
		let out_dir = config
			.workshop_artifacts_path
			.as_deref()
			.ok_or(DownloadWorkshopMapError::MissingWorkshopAssetDir)?;

		let depot_downloader_path = config
			.depot_downloader_path
			.as_deref()
			.ok_or(DownloadWorkshopMapError::MissingDepotDownloader)?;

		let output = Command::new(depot_downloader_path)
			.args(["-app", "730", "-pubfile"])
			.arg(workshop_id.to_string())
			.arg("-dir")
			.arg(out_dir)
			.spawn()
			.inspect_err(|err| {
				error!(target: "audit_log", %err, "failed to run DepotDownloader");
			})?
			.wait_with_output()
			.await
			.inspect_err(|err| {
				error!(target: "audit_log", %err, "failed to run DepotDownloader");
			})?;

		let mut stdout = io::stdout();
		let mut stderr = io::stderr();

		if let Err(err) = tokio::try_join!(stdout.flush(), stderr.flush()) {
			error!(target: "audit_log", %err, "failed to flush stdout/stderr");
		}

		if !output.status.success() {
			error!(target: "audit_log", ?output, "DepotDownloader did not exit successfully");
			return Err(io::Error::new(
				io::ErrorKind::Other,
				"DepotDownloader did not exit successfully",
			)
			.into());
		}

		let filepath = out_dir.join(format!("{workshop_id}.vpk"));
		let file = File::open(&filepath).await.inspect_err(|err| {
			error!(target: "audit_log", %err, ?filepath, "failed to open map file");
		})?;

		Ok(Self { file })
	}

	/// Calculate the checksum for this map file.
	///
	/// # Panics
	///
	/// This function will panic if the filesize exceeds `usize::MAX` bytes.
	pub async fn checksum(mut self) -> io::Result<u32> {
		let metadata = self.file.metadata().await?;
		let filesize = usize::try_from(metadata.len()).expect("64-bit platform");
		let mut buf = Vec::with_capacity(filesize);

		self.file.read_to_end(&mut buf).await.inspect_err(|err| {
			error!(target: "audit_log", %err, "failed to read map file");
		})?;

		Ok(crc32fast::hash(&buf))
	}
}

/// The different kinds of errors that can occur when downloading a Workshop map.
#[derive(Debug, Error)]
pub enum DownloadWorkshopMapError {
	/// Before downloading a map, we need to know where to store it.
	///
	/// This is controlled by an environment variable (which we don't enforce on startup).
	#[error("missing workshop asset directory; try setting `KZ_API_WORKSHOP_PATH`")]
	MissingWorkshopAssetDir,

	/// Before downloading a map, we need to know that `DepotDownloader` exists and where it is.
	///
	/// This is controlled by an environment variable (which we don't enforce on startup).
	#[error("missing `DepotDownloader` binary; try setting `DEPOT_DOWNLOADER_PATH`")]
	MissingDepotDownloader,

	/// An I/O operation failed, such as opening a file.
	#[error("failed to download workshop map: {0}")]
	Io(#[from] io::Error),
}
