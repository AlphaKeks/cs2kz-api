use std::path::Path;
use std::sync::LazyLock;
use std::{io, ops};

use derive_more::Debug;
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::sync::{Semaphore, SemaphorePermit};

/* N.B.
 *
 * We limit the max. number of open files and blocking threads across the entire application using
 * a semaphore, so frequent requests calling `SteamService::download_map()` don't overload the
 * system.
 *
 * This means we need to make sure to acquire a permit anytime we interact with the file handle.
 */

/// The maximum number of open files we want to allow.
const MAX_FILE_COUNT: usize = 32;

/// Semaphore limiting the total amount of open files in the program.
static FILE_PERMITS: LazyLock<Semaphore> = LazyLock::new(|| Semaphore::new(MAX_FILE_COUNT));

/// Acquires a permit from the semaphore above.
async fn acquire_file_permit() -> SemaphorePermit<'static>
{
	FILE_PERMITS
		.acquire()
		.await
		.expect("a static is never dropped")
}

/// A handle to an open map file.
#[must_use]
#[derive(Debug)]
pub struct MapHandle(File);

impl MapHandle
{
	/// Creates a new [`MapHandle`] by opening the file at the given `path`.
	pub async fn new(path: &Path) -> io::Result<Self>
	{
		let _permit = acquire_file_permit().await;
		File::open(path).await.map(Self)
	}

	/// Reads the map file into memory and computes its MD5 hash.
	pub async fn hash(&mut self) -> io::Result<MapFileHash>
	{
		let _permit = acquire_file_permit().await;
		let filesize = self.0.metadata().await.map(|metadata| metadata.len())?;
		let mut buf = Vec::with_capacity(filesize as usize);

		self.0.read_to_end(&mut buf).await?;

		Ok(MapFileHash(md5::compute(&buf).0))
	}
}

/// An MD5 hash of a map file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[debug("{}", hex::encode(_0))]
#[serde(transparent)]
pub struct MapFileHash(#[serde(with = "hex::serde")] [u8; 16]);

impl ops::Deref for MapFileHash
{
	type Target = [u8];

	fn deref(&self) -> &Self::Target
	{
		&self.0[..]
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for MapFileHash
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>>
	{
		let bytes = <&'r [u8]>::decode(value)?;

		bytes.try_into().map(Self).map_err(Into::into)
	}
}
