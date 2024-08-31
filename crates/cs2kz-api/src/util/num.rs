use std::num::NonZero;

use serde::{Deserialize, Deserializer};

pub fn deserialize_non_zero_usize_opt<'de, D>(
	deserializer: D,
) -> Result<Option<NonZero<usize>>, D::Error>
where
	D: Deserializer<'de>,
{
	Option::<usize>::deserialize(deserializer).map(|n| n.and_then(NonZero::new))
}

pub fn deserialize_non_zero_u32_opt<'de, D>(
	deserializer: D,
) -> Result<Option<NonZero<u32>>, D::Error>
where
	D: Deserializer<'de>,
{
	Option::<u32>::deserialize(deserializer).map(|n| n.and_then(NonZero::new))
}
