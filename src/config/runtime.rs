use {
	crate::runtime,
	serde::{Deserialize, Deserializer},
	std::num::NonZero,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "kebab-case")]
pub(crate) struct RuntimeConfig
{
	#[serde(default)]
	pub environment: runtime::Environment,

	#[serde(default, deserialize_with = "deserialize_option_usize")]
	pub worker_threads: Option<NonZero<usize>>,

	#[serde(default, deserialize_with = "deserialize_option_usize")]
	pub max_blocking_threads: Option<NonZero<usize>>,
}

fn deserialize_option_usize<'de, D>(deserializer: D) -> Result<Option<NonZero<usize>>, D::Error>
where
	D: Deserializer<'de>,
{
	<Option<usize> as Deserialize<'de>>::deserialize(deserializer)
		.map(|maybe_num| maybe_num.and_then(NonZero::new))
}
