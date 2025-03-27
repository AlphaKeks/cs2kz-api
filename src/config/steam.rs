use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct SteamConfig
{
	pub api_key: Box<str>,
}
