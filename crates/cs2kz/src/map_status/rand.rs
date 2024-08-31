//! Trait implementations for the [`rand`] crate.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

use super::MapState;

impl Distribution<MapState> for Standard
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> MapState
	{
		rng.gen_range(-1..=1)
			.try_into()
			.expect("any integer between -1 and 1 is a valid map state")
	}
}
