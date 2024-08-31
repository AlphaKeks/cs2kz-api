//! Iterator types & implementations for [`Styles`].

use super::{Style, Styles};

/// An iterator over [Styles], yielding each set bit as a [`Style`].
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq)]
pub struct StylesIter
{
	/// The bits we haven't yielded yet.
	bits: u32,
}

impl StylesIter
{
	/// Creates a new [`StylesIter`].
	pub(super) fn new(styles: Styles) -> Self
	{
		Self { bits: styles.0 }
	}
}

impl Iterator for StylesIter
{
	type Item = Style;

	fn next(&mut self) -> Option<Self::Item>
	{
		while self.bits != 0 {
			let lsb = 1 << self.bits.trailing_zeros();
			self.bits &= !lsb;

			match Styles::from_u32(lsb) {
				Ok(Styles::AUTO_BHOP) => return Some(Style::AutoBhop),
				Ok(_) => unreachable!("invalid bit found in styles"),
				Err(_) => continue,
			}
		}

		None
	}
}

impl IntoIterator for Styles
{
	type Item = Style;
	type IntoIter = StylesIter;

	fn into_iter(self) -> Self::IntoIter
	{
		StylesIter::new(self)
	}
}

impl FromIterator<Style> for Styles
{
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Style>,
	{
		iter.into_iter().fold(Self::NONE, Self::with_style)
	}
}
