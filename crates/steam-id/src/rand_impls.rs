use ::rand::distr::{Distribution, StandardUniform};
use rand::Rng;

use crate::{AccountNumber, AccountUniverse, SteamId};

impl Distribution<SteamId> for StandardUniform
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> SteamId
	{
		SteamId::from_parts(
			AccountUniverse::Public,
			AccountNumber::from_bits(u64::from(rng.random::<u32>())),
			rng.random::<bool>(),
		)
	}
}
