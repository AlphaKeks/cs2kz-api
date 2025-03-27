macro_rules! impl_rand {
	($ty:ty => |$rng:pat_param| $impl:expr) => {
		#[cfg(feature = "rand")]
		impl ::rand::distr::Distribution<$ty> for ::rand::distr::StandardUniform
		{
			fn sample<R: ::rand::Rng + ?Sized>(&self, $rng: &mut R) -> $ty
			{
				$impl
			}
		}
	};
}
