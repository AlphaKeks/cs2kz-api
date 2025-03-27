use std::{fmt, ops};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeSeq};
use utoipa::ToSchema;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, try_transmute_ref};

const AUTO_BHOP: u32 = 1_u32 << 0;

#[repr(u32)]
#[non_exhaustive]
#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	TryFromBytes,
	IntoBytes,
	Immutable,
	KnownLayout,
	Serialize,
	Deserialize,
	ToSchema,
	sqlx::Type,
)]
#[serde(rename_all = "kebab-case")]
pub enum Style
{
	AutoBhop = AUTO_BHOP,
}

/// A set of [`Style`]s
#[derive(
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	TryFromBytes,
	IntoBytes,
	Immutable,
	KnownLayout,
	sqlx::Type,
	ToSchema,
)]
#[sqlx(transparent)]
#[schema(value_type = [Style])]
pub struct Styles(u32);

/// An [`Iterator`] over [`Style`]s
#[derive(Debug, Clone)]
pub struct Iter
{
	bits: u32,
}

impl Styles
{
	/// Returns the number of [`Style`]s stored in `self`.
	pub fn count(&self) -> u32
	{
		self.0.count_ones()
	}

	pub fn is_empty(&self) -> bool
	{
		self.0 == 0
	}

	/// Checks if `other` is a subset of `self`.
	pub fn contains<P>(&self, other: &P) -> bool
	where
		P: ?Sized + AsRef<Styles>,
	{
		let other = other.as_ref();
		(self.0 & other.0) == other.0
	}

	/// Returns an [`Iterator`] over the [`Style`]s stored in `self`.
	pub fn iter(&self) -> Iter
	{
		Iter { bits: self.0 }
	}
}

impl AsRef<Styles> for Style
{
	fn as_ref(&self) -> &Styles
	{
		try_transmute_ref!(self).unwrap_or_else(|err| {
			panic!("conversions from `Style` to `Styles` should always succeed\n{err}");
		})
	}
}

impl ops::BitAnd for Style
{
	type Output = Styles;

	fn bitand(self, rhs: Style) -> Self::Output
	{
		Styles((self as u32) & (rhs as u32))
	}
}

impl ops::BitAnd<Styles> for Style
{
	type Output = Styles;

	fn bitand(self, rhs: Styles) -> Self::Output
	{
		Styles((self as u32) & (rhs.0))
	}
}

impl ops::BitOr for Style
{
	type Output = Styles;

	fn bitor(self, rhs: Style) -> Self::Output
	{
		Styles((self as u32) | (rhs as u32))
	}
}

impl ops::BitOr<Styles> for Style
{
	type Output = Styles;

	fn bitor(self, rhs: Styles) -> Self::Output
	{
		Styles((self as u32) | (rhs.0))
	}
}

impl ops::BitXor for Style
{
	type Output = Styles;

	fn bitxor(self, rhs: Style) -> Self::Output
	{
		Styles((self as u32) ^ (rhs as u32))
	}
}

impl ops::BitXor<Styles> for Style
{
	type Output = Styles;

	fn bitxor(self, rhs: Styles) -> Self::Output
	{
		Styles((self as u32) ^ (rhs.0))
	}
}

impl fmt::Debug for Styles
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_set().entries(self).finish()
	}
}

impl From<Style> for Styles
{
	fn from(style: Style) -> Self
	{
		Self(style as u32)
	}
}

impl IntoIterator for &Styles
{
	type Item = Style;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		self.iter()
	}
}

impl IntoIterator for Styles
{
	type Item = Style;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		self.iter()
	}
}

impl Serialize for Styles
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(Some(self.count() as usize))?;

		for style in self {
			serializer.serialize_element(&style)?;
		}

		serializer.end()
	}
}

impl<'de> Deserialize<'de> for Styles
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct StylesVisitor;

		impl<'de> de::Visitor<'de> for StylesVisitor
		{
			type Value = Styles;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
			{
				fmt.write_str("a list of user styles")
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				let mut styles = Styles::default();

				while let Some(style) = seq.next_element::<Style>()? {
					styles |= style;
				}

				Ok(styles)
			}
		}

		deserializer.deserialize_seq(StylesVisitor)
	}
}

impl ops::BitAnd for Styles
{
	type Output = Styles;

	fn bitand(self, rhs: Styles) -> Self::Output
	{
		Styles((self.0) & (rhs.0))
	}
}

impl ops::BitAnd<Style> for Styles
{
	type Output = Styles;

	fn bitand(self, rhs: Style) -> Self::Output
	{
		Styles((self.0) & (rhs as u32))
	}
}

impl ops::BitAndAssign for Styles
{
	fn bitand_assign(&mut self, rhs: Styles)
	{
		self.0 &= rhs.0;
	}
}

impl ops::BitAndAssign<Style> for Styles
{
	fn bitand_assign(&mut self, rhs: Style)
	{
		self.0 &= rhs as u32;
	}
}

impl ops::BitOr for Styles
{
	type Output = Styles;

	fn bitor(self, rhs: Styles) -> Self::Output
	{
		Styles((self.0) | (rhs.0))
	}
}

impl ops::BitOr<Style> for Styles
{
	type Output = Styles;

	fn bitor(self, rhs: Style) -> Self::Output
	{
		Styles((self.0) | (rhs as u32))
	}
}

impl ops::BitOrAssign for Styles
{
	fn bitor_assign(&mut self, rhs: Styles)
	{
		self.0 |= rhs.0;
	}
}

impl ops::BitOrAssign<Style> for Styles
{
	fn bitor_assign(&mut self, rhs: Style)
	{
		self.0 |= rhs as u32;
	}
}

impl ops::BitXor for Styles
{
	type Output = Styles;

	fn bitxor(self, rhs: Styles) -> Self::Output
	{
		Styles((self.0) ^ (rhs.0))
	}
}

impl ops::BitXor<Style> for Styles
{
	type Output = Styles;

	fn bitxor(self, rhs: Style) -> Self::Output
	{
		Styles((self.0) ^ (rhs as u32))
	}
}

impl ops::BitXorAssign for Styles
{
	fn bitxor_assign(&mut self, rhs: Styles)
	{
		self.0 ^= rhs.0;
	}
}

impl ops::BitXorAssign<Style> for Styles
{
	fn bitxor_assign(&mut self, rhs: Style)
	{
		self.0 ^= rhs as u32;
	}
}

impl FromIterator<Style> for Styles
{
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Style>,
	{
		iter.into_iter().fold(Self::default(), ops::BitOr::bitor)
	}
}

impl Iterator for Iter
{
	type Item = Style;

	fn next(&mut self) -> Option<Self::Item>
	{
		if self.bits == 0 {
			return None;
		}

		let next_bit = 1 << self.bits.trailing_zeros();
		self.bits &= !next_bit;

		Style::try_read_from_bytes(next_bit.as_bytes())
			.map_or_else(|err| panic!("invalid style bit in `StyleIter`\n{err}"), Some)
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		let count = self.bits.count_ones() as usize;
		(count, Some(count))
	}

	fn count(self) -> usize
	where
		Self: Sized,
	{
		self.bits.count_ones() as usize
	}
}

impl ExactSizeIterator for Iter
{
}
