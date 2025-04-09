use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct AccessKeys
{
	/// The name of the key used for publishing new releases of [`cs2kz-metamod`] via GitHub
	/// Actions.
	///
	/// [`cs2kz-metamod`]: https://github.com/KZGlobalTeam/cs2kz-metamod
	#[serde(default = "default_cs2kz_metamod_release_key")]
	pub cs2kz_metamod_release_key: Box<str>,
}

impl Default for AccessKeys
{
	fn default() -> Self
	{
		Self { cs2kz_metamod_release_key: default_cs2kz_metamod_release_key() }
	}
}

fn default_cs2kz_metamod_release_key() -> Box<str>
{
	Box::from("github:cs2kz-metamod:release")
}
