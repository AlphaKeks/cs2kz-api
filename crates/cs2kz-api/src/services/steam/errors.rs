use std::io;

use problem_details::AsProblemDetails;

use crate::http::Problem;

/// Errors that can occur when fetching a user from Steam.
#[derive(Debug, Error)]
pub enum GetUserError
{
	/// The HTTP request failed in some way.
	#[error(transparent)]
	Http(#[from] reqwest::Error),
}

impl AsProblemDetails for GetUserError
{
	type ProblemType = Problem;

	fn problem_type(&self) -> Self::ProblemType
	{
		match self {
			Self::Http(error) => error
				.status()
				.filter(|status| status.is_client_error())
				.map_or(Problem::ExternalService, |_| Problem::ResourceNotFound),
		}
	}
}

/// Errors that can occur when fetching a map's name from Steam.
#[derive(Debug, Error)]
pub enum GetMapNameError
{
	/// The HTTP request failed in some way.
	#[error(transparent)]
	Http(#[from] reqwest::Error),
}

/// Errors that can occur when downloading a workshop map.
#[derive(Debug, Error)]
pub enum DownloadMapError
{
	/// We failed to spawn the child process for `DepotDownloader`.
	#[error("failed to spawn DepotDownloader child process")]
	SpawnChild(#[source] io::Error),

	/// The `DepotDownloader` child process exited with a non-zero exit code.
	#[error("DepotDownloader exited with a non-zero exit code")]
	NonZeroExitCode,

	/// We failed to wait for the `DepotDownloader` child process to finish.
	#[error("failed to wait for DepotDownloader child process")]
	WaitForChild,

	/// We failed to open the downloaded map file.
	#[error("failed to open map file")]
	OpenMapFile(#[source] io::Error),
}
