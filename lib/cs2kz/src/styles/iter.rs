//! [`Iterator`]s for [`Styles`].

use super::Styles;

/// An iterator over the multiple styles contained in a [`Styles`] bitflags set.
#[derive(Debug, Clone)]
pub struct Iter
{
	/// The bits that are left.
	bits: u32,

	/// The current bit index we are at.
	idx: u32,
}

impl Iter
{
	/// Creates a new [`Iter`].
	const fn new(styles: Styles) -> Self
	{
		Self { bits: styles.bits(), idx: 0 }
	}
}

impl Iterator for Iter
{
	type Item = &'static str;

	fn next(&mut self) -> Option<Self::Item>
	{
		if self.bits == 0 {
			return None;
		}

		if self.idx >= u32::BITS {
			return None;
		}

		while self.bits != 0 && self.idx < u32::BITS {
			if let Some(name) = Styles::new(self.bits & (1 << self.idx)).name() {
				self.idx += 1;
				return Some(name);
			}

			self.idx += 1;
		}

		None
	}
}

impl IntoIterator for Styles
{
	type Item = &'static str;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		Iter::new(self)
	}
}

impl IntoIterator for &Styles
{
	type Item = &'static str;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		Iter::new(*self)
	}
}
