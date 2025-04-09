use {
	crate::{AccountNumber, AccountUniverse, SteamId},
	::rand::distr::{Distribution, StandardUniform},
	rand::Rng,
};

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
