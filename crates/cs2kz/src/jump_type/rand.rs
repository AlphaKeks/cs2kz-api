//! Trait implementations for the [`rand`] crate.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

use super::JumpType;

impl Distribution<JumpType> for Standard
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> JumpType
	{
		rng.gen_range(1..=7)
			.try_into()
			.expect("any integer between 1 and 7 is a valid jump type")
	}
}
