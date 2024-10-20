use serde::de::{Deserialize, Deserializer, Error};

/// Deserializes a value of type `T`, validating that it isn't empty.
#[expect(private_bounds, reason = "`Length` is an implementation detail")]
#[expect(dead_code)]
pub fn deserialize_non_empty<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
	T: Deserialize<'de> + Length,
	D: Deserializer<'de>,
{
	let value = T::deserialize(deserializer)?;

	if value.length() == 0 {
		return Err(Error::invalid_length(0, &"1 or more"));
	}

	Ok(value)
}

/// Helper trait for the `deserialize_non_empty` function above.
///
/// Many collection types (`Vec`, `String`, `HashSet`, etc.) have a notion of "length" or "size",
/// usually in the form of a `.len()` method. The standard library does not have a generic
/// abstraction over this notion, so this little helper trait bridges the gap.
///
/// It is meant to be used with `deserialize_non_empty`, so if you find yourself using that
/// function, but getting a compiler error regarding `Length`, consider just implementing this
/// trait for whatever type you're using.
///
/// The macro below should take care of anything that has a `.len()` method.
trait Length {
	fn length(&self) -> usize;
}

macro_rules! impl_length {
	( $($ty:ty),* $(,)? ) => { $(
		impl Length for $ty {
			fn length(&self) -> usize {
				self.len()
			}
		}
	)* };
}

impl_length![String];
