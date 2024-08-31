//! Trait implementations for the [`rand`] crate.

use rand::distributions::{Distribution, Standard};
use rand::Rng;

use super::Mode;

impl Distribution<Mode> for Standard
{
	fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Mode
	{
		if <Standard as Distribution<bool>>::sample(self, rng) {
			Mode::Vanilla
		} else {
			Mode::Classic
		}
	}
}
