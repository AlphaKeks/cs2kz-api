use std::num::NonZero;

/// Tokio configuration.
#[derive(Default, Debug, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RuntimeConfig {
	/// The amount of worker threads to spawn.
	///
	/// If this value is omitted or 0, tokio will choose the amount.
	#[serde(default, deserialize_with = "deserialize_worker_threads")]
	pub worker_threads: Option<NonZero<usize>>,

	/// The maximum amount of threads to spawn in the blocking thread pool.
	///
	/// If this value is omitted or 0, tokio will choose the amount.
	#[serde(default, deserialize_with = "deserialize_worker_threads")]
	pub max_blocking_threads: Option<NonZero<usize>>,
}

fn deserialize_worker_threads<'de, D>(deserializer: D) -> Result<Option<NonZero<usize>>, D::Error>
where
	D: serde::Deserializer<'de>,
{
	<usize as serde::Deserialize>::deserialize(deserializer).map(NonZero::new)
}
