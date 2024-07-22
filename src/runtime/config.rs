//! Runtime configuration for the API.
//!
//! This module contains the [`Config`] struct - a set of configuration options
//! that will be read from the environment on startup. See the `.env.example`
//! file in the root of the repository for examples.

use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fmt};

use thiserror::Error;
use url::Url;

/// The API's runtime configuration.
///
/// See [module level docs] for more details.
///
/// [module level docs]: crate::runtime::config
#[derive(Clone)]
pub struct Config
{
	/// [`Inner`] actually contains all the values, we just wrap it so
	/// [`Config`] is cheap to clone.
	inner: Arc<Inner>,
}

/// Error that can occur while initializing the API's [`Config`].
#[derive(Debug, Error)]
pub enum InitializeConfigError
{
	/// A required environment variable was not found or invalid
	/// UTF-8.
	#[error("failed to read configuration value: {0}")]
	Env(#[from] env::VarError),

	/// A required configuration option was empty.
	#[error("`{0}` cannot be empty")]
	EmptyValue(&'static str),

	/// A required configuration option could not be parsed into the required
	/// type.
	#[error("failed to parse configuration value: {0}")]
	Parse(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl Config
{
	/// Initializes a [`Config`] by reading and parsing environment variables.
	pub fn new() -> Result<Self, InitializeConfigError>
	{
		let public_url = parse_from_env::<Url>("KZ_API_PUBLIC_URL")?;
		let database_url = parse_from_env::<Url>("DATABASE_URL")?;
		let cookie_domain = parse_from_env::<String>("KZ_API_COOKIE_DOMAIN")?;
		let steam_api_key = parse_from_env::<String>("STEAM_WEB_API_KEY")?;
		let jwt_secret = parse_from_env::<String>("KZ_API_JWT_SECRET")?;

		#[cfg(feature = "production")]
		let workshop_artifacts_path = parse_from_env::<PathBuf>("KZ_API_WORKSHOP_PATH")?;

		#[cfg(not(feature = "production"))]
		let workshop_artifacts_path = parse_from_env_opt::<PathBuf>("KZ_API_WORKSHOP_PATH")?;

		#[cfg(feature = "production")]
		let depot_downloader_path = parse_from_env::<PathBuf>("DEPOT_DOWNLOADER_PATH")?;

		#[cfg(not(feature = "production"))]
		let depot_downloader_path = parse_from_env_opt::<PathBuf>("DEPOT_DOWNLOADER_PATH")?;

		Ok(Self {
			inner: Arc::new(Inner {
				public_url,
				database_url,
				cookie_domain,
				steam_api_key,
				jwt_secret,
				workshop_artifacts_path,
				depot_downloader_path,
			}),
		})
	}

	/// Returns the API's public URL.
	pub fn public_url(&self) -> &Url
	{
		&self.inner.public_url
	}

	/// Returns the API's database URL.
	pub fn database_url(&self) -> &Url
	{
		&self.inner.database_url
	}

	pub fn cookie_domain(&self) -> &str
	{
		&self.inner.cookie_domain
	}

	pub fn steam_api_key(&self) -> &str
	{
		&self.inner.steam_api_key
	}

	pub fn jwt_secret(&self) -> &str
	{
		&self.inner.jwt_secret
	}

	#[cfg(feature = "production")]
	pub fn workshop_artifacts_path(&self) -> &Path
	{
		&self.inner.workshop_artifacts_path
	}

	#[cfg(not(feature = "production"))]
	pub fn workshop_artifacts_path(&self) -> Option<&Path>
	{
		self.inner.workshop_artifacts_path.as_deref()
	}

	#[cfg(feature = "production")]
	pub fn depot_downloader_path(&self) -> &Path
	{
		&self.inner.depot_downloader_path
	}

	#[cfg(not(feature = "production"))]
	pub fn depot_downloader_path(&self) -> Option<&Path>
	{
		self.inner.depot_downloader_path.as_deref()
	}
}

impl fmt::Debug for Config
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_struct("Config")
			.field("public_url", &format_args!("{:?}", self.public_url().as_str()))
			.field("database_url", &format_args!("{:?}", self.database_url().as_str()))
			.field("cookie_domain", &self.cookie_domain())
			.field("steam_api_key", &self.steam_api_key())
			.field("jwt_secret", &self.jwt_secret())
			.field("workshop_artifacts_path", &self.workshop_artifacts_path())
			.field("depot_downloader_path", &self.depot_downloader_path())
			.finish_non_exhaustive()
	}
}

#[allow(clippy::missing_docs_in_private_items)]
struct Inner
{
	/// The URL at which the API will be reachable.
	public_url: Url,

	/// The URL of the API's database.
	database_url: Url,

	/// `Domain` field to set for HTTP cookies.
	cookie_domain: String,

	/// Steam WebAPI key for making HTTP requests to Steam.
	///
	/// Get yours here: <https://steamcommunity.com/dev/apikey>
	steam_api_key: String,

	/// Base64 secret for encoding/decoding JWTs.
	jwt_secret: String,

	/// Path to a directory storing Steam Workshop download artifacts.
	#[cfg(feature = "production")]
	workshop_artifacts_path: PathBuf,

	/// Path to a directory storing Steam Workshop download artifacts.
	#[cfg(not(feature = "production"))]
	workshop_artifacts_path: Option<PathBuf>,

	/// Path to the [DepotDownloader] executable.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(feature = "production")]
	depot_downloader_path: PathBuf,

	/// Path to the [DepotDownloader] executable.
	///
	/// [DepotDownloader]: https://github.com/SteamRE/DepotDownloader
	#[cfg(not(feature = "production"))]
	depot_downloader_path: Option<PathBuf>,
}

/// Reads and parses an environment variable.
fn parse_from_env<T>(var: &'static str) -> Result<T, InitializeConfigError>
where
	T: FromStr<Err: std::error::Error + Send + Sync + 'static>,
{
	let value = env::var(var)?;

	if value.is_empty() {
		return Err(InitializeConfigError::EmptyValue(var));
	}

	value
		.parse::<T>()
		.map_err(|error| InitializeConfigError::Parse(Box::new(error)))
}

/// Reads and parses an environment variable.
///
/// Returns [`None`] if a variable does not exist or is empty.
#[cfg(not(feature = "production"))]
fn parse_from_env_opt<T>(var: &'static str) -> Result<Option<T>, InitializeConfigError>
where
	T: FromStr<Err: std::error::Error + Send + Sync + 'static>,
{
	let Some(value) = env::var(var).ok() else {
		return Ok(None);
	};

	if value.is_empty() {
		return Ok(None);
	}

	value
		.parse::<T>()
		.map(Some)
		.map_err(|error| InitializeConfigError::Parse(Box::new(error)))
}
