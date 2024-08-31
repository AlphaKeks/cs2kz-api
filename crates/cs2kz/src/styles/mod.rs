//! The official gameplay styles available in CS2KZ.

use std::fmt;

mod errors;
pub use errors::InvalidBit;

mod style;
pub use style::{Style, UnknownStyle};

mod iter;
pub use iter::StylesIter;

cfg_sqlx! {
	mod sqlx;
}

/// A set of [styles].
///
/// The styles are combined into a single integer as bitflags.
///
/// [styles]: Style
#[repr(transparent)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Styles(u32);

impl Styles
{
	/// No styles.
	pub const NONE: Self = Self(0);

	/// The ABH style.
	pub const AUTO_BHOP: Self = Self(1 << 0);

	/// All styles.
	// N.B. update this when new styles are added!
	pub const ALL: Self = Self(1 << 0);

	/// Returns the underlying integer.
	pub const fn as_u32(self) -> u32
	{
		self.0
	}

	/// Constructs a [`Styles`] value from an integer.
	///
	/// For an infallible version of this constructor, see [`from_u32_lossy`].
	///
	/// [`from_u32_lossy`]: Self::from_u32_lossy
	pub const fn from_u32(value: u32) -> Result<Self, InvalidBit>
	{
		let styles = Self::from_u32_lossy(value);

		if styles.0 == value {
			Ok(styles)
		} else {
			Err(InvalidBit)
		}
	}

	/// Constructs a [`Styles`] value from an integer, ignoring invalid bits.
	pub const fn from_u32_lossy(value: u32) -> Self
	{
		Self(value & Self::ALL.0)
	}

	/// Adds the given `style` to this style set.
	pub const fn with_style(self, style: Style) -> Self
	{
		Self(self.0 | style.as_u32())
	}
}

impl fmt::Display for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		f.debug_list().entries(*self).finish()
	}
}

impl fmt::Binary for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for Styles
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl From<Styles> for u32
{
	fn from(Styles(bits): Styles) -> Self
	{
		bits
	}
}

impl TryFrom<u32> for Styles
{
	type Error = InvalidBit;

	fn try_from(bits: u32) -> Result<Self, Self::Error>
	{
		Self::from_u32(bits)
	}
}
