use std::num::NonZero;

use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config
{
	/// Maximum number of threads to spawn in the blocking thread pool.
	///
	/// See [`tokio::runtime::Builder::max_blocking_threads()`].
	#[serde(
		default,
		deserialize_with = "crate::util::num::deserialize_non_zero_usize_opt"
	)]
	pub max_blocking_threads: Option<NonZero<usize>>,

	/// Name to use for worker threads.
	///
	/// See [`tokio::runtime::Builder::thread_name()`].
	#[serde(default, with = "crate::util::non_empty::option")]
	pub worker_thread_name: Option<Box<str>>,

	/// Stack size (in bytes) for worker threads.
	///
	/// See [`tokio::runtime::Builder::thread_stack_size()`].
	#[serde(
		default,
		deserialize_with = "crate::util::num::deserialize_non_zero_usize_opt"
	)]
	pub worker_thread_stack_size: Option<NonZero<usize>>,

	/// Number of worker threads to spawn.
	///
	/// See [`tokio::runtime::Builder::worker_threads()`].
	#[serde(
		default,
		deserialize_with = "crate::util::num::deserialize_non_zero_usize_opt"
	)]
	pub worker_thread_count: Option<NonZero<usize>>,
}
