//! Trait implementations for the [`rand`] crate.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

use super::Tier;

impl Distribution<Tier> for Standard
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Tier
	{
		rng.gen_range(1..=10)
			.try_into()
			.expect("any integer between 1 and 10 is a valid tier")
	}
}
