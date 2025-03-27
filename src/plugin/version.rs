use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
	Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema,
)]
#[serde(transparent)]
#[schema(value_type = str, example = "1.23.456-dev")]
pub struct PluginVersion(semver::Version);

impl PluginVersion
{
	pub const ZERO: Self = Self(semver::Version {
		major: 0,
		minor: 0,
		patch: 0,
		pre: semver::Prerelease::EMPTY,
		build: semver::BuildMetadata::EMPTY,
	});

	#[track_caller]
	pub fn from_parts(major: u64, minor: u64, patch: u64, pre: &str, build: &str) -> Self
	{
		let pre = pre.parse::<semver::Prerelease>().unwrap_or_else(|err| {
			panic!("invalid pre-release {pre:?}: {err}");
		});

		let build = build.parse::<semver::BuildMetadata>().unwrap_or_else(|err| {
			panic!("invalid build meta {build:?}: {err}");
		});

		Self(semver::Version { major, minor, patch, pre, build })
	}

	pub fn major(&self) -> u64
	{
		self.0.major
	}

	pub fn minor(&self) -> u64
	{
		self.0.minor
	}

	pub fn patch(&self) -> u64
	{
		self.0.patch
	}

	pub fn pre(&self) -> &str
	{
		self.0.pre.as_str()
	}

	pub fn build(&self) -> &str
	{
		self.0.build.as_str()
	}
}
