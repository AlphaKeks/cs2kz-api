#[derive(Default, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SteamConfig {
	/// Steam WebAPI Key.
	#[serde(default, deserialize_with = "deserialize_api_key")]
	pub api_key: Option<Box<str>>,
}

fn deserialize_api_key<'de, D>(deserializer: D) -> Result<Option<Box<str>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	<Option<Box<str>> as serde::Deserialize>::deserialize(deserializer)
		.map(|opt| opt.filter(|s| !s.is_empty()))
}
