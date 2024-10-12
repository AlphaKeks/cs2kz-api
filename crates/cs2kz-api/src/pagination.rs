use std::cmp;

use crate::database;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct Limit<const DEFAULT: u64 = { u64::MAX }, const MAX: u64 = { u64::MAX }>(u64);

impl<const DEFAULT: u64, const MAX: u64> Limit<DEFAULT, MAX> {
	pub fn new(value: u64) -> Self {
		Self(cmp::min(value, MAX))
	}
}

impl<const DEFAULT: u64, const MAX: u64> Default for Limit<DEFAULT, MAX> {
	fn default() -> Self {
		Self(DEFAULT)
	}
}

impl<'de, const DEFAULT: u64, const MAX: u64> serde::Deserialize<'de> for Limit<DEFAULT, MAX> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		Option::<u64>::deserialize(deserializer)
			.map(|maybe_limit| maybe_limit.map_or_else(Self::default, Self::new))
	}
}

database::macros::wrap!(Limit as u64);

#[derive(
	Debug,
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	Into,
	serde::Deserialize,
	sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Offset(i64);

#[derive(Debug)]
pub struct PaginationResults<S> {
	/// The total amount of results that _could_ have been returned if there were no limits.
	pub total: u64,

	/// The stream of results.
	#[debug("<stream>")]
	pub stream: S,
}
