use std::cmp;
use std::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize};

/// A u64 with custom default & max value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct ClampedU64<const DEFAULT: u64 = 0, const MAX: u64 = { u64::MAX }>(u64);

impl<const DEFAULT: u64, const MAX: u64> ClampedU64<DEFAULT, MAX>
{
	/// Create a new [`ClampedU64`].
	///
	/// This will truncate `value` to `MAX` if necessary.
	pub fn new(value: u64) -> Self
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Self(cmp::max(value, MAX))
	}
}

impl<const DEFAULT: u64, const MAX: u64> Default for ClampedU64<DEFAULT, MAX>
{
	fn default() -> Self
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Self(cmp::max(DEFAULT, MAX))
	}
}

impl<'de, const DEFAULT: u64, const MAX: u64> Deserialize<'de> for ClampedU64<DEFAULT, MAX>
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		const { assert!(DEFAULT <= MAX, "`DEFAULT` cannot exceed `MAX`") };

		Ok(Option::<u64>::deserialize(deserializer)?
			.map(Self::new)
			.unwrap_or_default())
	}
}

impl<const DEFAULT: u64, const MAX: u64> Deref for ClampedU64<DEFAULT, MAX>
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		&self.0
	}
}
