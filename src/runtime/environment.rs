use std::sync::OnceLock;

use serde::Deserialize;

static CURRENT: OnceLock<Environment> = OnceLock::new();

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, FromStr, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Environment
{
	Development,
	Testing,

	#[default]
	Production,
}

impl Environment
{
	/// Returns `true` if the environment is [`Development`].
	///
	/// [`Development`]: Environment::Development
	#[must_use]
	pub(crate) fn is_development(&self) -> bool
	{
		matches!(self, Self::Development)
	}

	/// Returns `true` if the environment is [`Testing`].
	///
	/// [`Testing`]: Environment::Testing
	#[must_use]
	pub(crate) fn is_testing(&self) -> bool
	{
		matches!(self, Self::Testing)
	}

	/// Returns `true` if the environment is [`Production`].
	///
	/// [`Production`]: Environment::Production
	#[must_use]
	pub(crate) fn is_production(&self) -> bool
	{
		matches!(self, Self::Production)
	}
}

#[track_caller]
pub(crate) fn get() -> Environment
{
	CURRENT.get().copied().unwrap_or_else(|| {
		panic!("attempted to get runtime environment before it was set");
	})
}

pub(crate) fn set(value: Environment) -> Result<Environment, Environment>
{
	CURRENT.try_insert(value).copied().map_err(|(&current, _)| current)
}
