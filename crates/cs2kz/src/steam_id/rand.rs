//! Trait implementations for the [`rand`] crate.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

use super::{SteamID, MAX, MIN};

impl Distribution<SteamID> for Standard
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SteamID
	{
		rng.gen_range(MIN..=MAX)
			.try_into()
			.expect("rng generated out-of-range value")
	}
}
