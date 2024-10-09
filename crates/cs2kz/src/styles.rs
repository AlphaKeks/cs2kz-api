use std::str::FromStr;
use std::{fmt, ops};

/// The different gameplay styles in CS2KZ.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Style {
	/// The ABH style.
	AutoBhop = 1 << 0,
}

/// 0 or more [`Style`]s combined as bitflags.
#[repr(transparent)]
#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Styles(u32);

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown style")]
pub struct UnknownStyle;

impl Styles {
	/// Creates a new, empty set of [`Style`]s.
	pub const fn new() -> Self {
		Self(0)
	}

	/// Creates a new set of [`Style`]s containing all available styles.
	pub const fn all() -> Self {
		Self(Style::AutoBhop as u32)
	}

	/// Returns the raw integer inside this set.
	pub const fn as_u32(self) -> u32 {
		self.0
	}

	/// Checks if the given `style` is in `self`.
	pub const fn contains(self, style: Style) -> bool {
		(self.0 & (style as u32)) == (style as u32)
	}

	/// Creates an iterator over the [`Style`]s stored in this set.
	pub const fn iter(&self) -> StylesIter {
		StylesIter::new(*self)
	}
}

impl fmt::Display for Style {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Style::AutoBhop => "ABH",
		})
	}
}

impl fmt::Debug for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_list().entries(self).finish()
	}
}

impl fmt::Display for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self, f)
	}
}

impl fmt::Binary for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for Styles {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Octal::fmt(&self.0, f)
	}
}

impl ops::BitAnd for Styles {
	type Output = Styles;

	fn bitand(self, rhs: Styles) -> Self::Output {
		Styles(self.0 & rhs.0)
	}
}

impl ops::BitAnd<Style> for Styles {
	type Output = Styles;

	fn bitand(self, rhs: Style) -> Self::Output {
		Styles(self.0 & (rhs as u32))
	}
}

impl ops::BitAnd<Styles> for Style {
	type Output = Styles;

	fn bitand(self, rhs: Styles) -> Self::Output {
		Styles(rhs.0 & (self as u32))
	}
}

impl ops::BitAndAssign for Styles {
	fn bitand_assign(&mut self, rhs: Styles) {
		self.0 &= rhs.0;
	}
}

impl ops::BitAndAssign<Style> for Styles {
	fn bitand_assign(&mut self, rhs: Style) {
		self.0 &= rhs as u32;
	}
}

impl ops::BitOr for Styles {
	type Output = Styles;

	fn bitor(self, rhs: Styles) -> Self::Output {
		Styles(self.0 | rhs.0)
	}
}

impl ops::BitOr<Style> for Styles {
	type Output = Styles;

	fn bitor(self, rhs: Style) -> Self::Output {
		Styles(self.0 | (rhs as u32))
	}
}

impl ops::BitOr<Styles> for Style {
	type Output = Styles;

	fn bitor(self, rhs: Styles) -> Self::Output {
		Styles(rhs.0 | (self as u32))
	}
}

impl ops::BitOrAssign for Styles {
	fn bitor_assign(&mut self, rhs: Styles) {
		self.0 |= rhs.0;
	}
}

impl ops::BitOrAssign<Style> for Styles {
	fn bitor_assign(&mut self, rhs: Style) {
		self.0 |= rhs as u32;
	}
}

impl ops::BitXor for Styles {
	type Output = Styles;

	fn bitxor(self, rhs: Styles) -> Self::Output {
		Styles(self.0 ^ rhs.0)
	}
}

impl ops::BitXor<Style> for Styles {
	type Output = Styles;

	fn bitxor(self, rhs: Style) -> Self::Output {
		Styles(self.0 ^ (rhs as u32))
	}
}

impl ops::BitXor<Styles> for Style {
	type Output = Styles;

	fn bitxor(self, rhs: Styles) -> Self::Output {
		Styles(rhs.0 ^ (self as u32))
	}
}

impl ops::BitXorAssign for Styles {
	fn bitxor_assign(&mut self, rhs: Styles) {
		self.0 ^= rhs.0;
	}
}

impl ops::BitXorAssign<Style> for Styles {
	fn bitxor_assign(&mut self, rhs: Style) {
		self.0 ^= rhs as u32;
	}
}

impl From<Style> for u32 {
	fn from(style: Style) -> Self {
		style as u32
	}
}

impl TryFrom<u32> for Style {
	type Error = UnknownStyle;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		const ABH: u32 = Style::AutoBhop as u32;

		match value {
			ABH => Ok(Style::AutoBhop),
			_ => Err(UnknownStyle),
		}
	}
}

impl From<Styles> for u32 {
	fn from(styles: Styles) -> Self {
		styles.0
	}
}

impl FromStr for Style {
	type Err = UnknownStyle;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(value) = value.parse::<u32>() {
			return Self::try_from(value);
		}

		match value {
			"abh" | "ABH" | "auto_bhop" => Ok(Style::AutoBhop),
			_ => Err(UnknownStyle),
		}
	}
}

impl FromIterator<Style> for Styles {
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Style>,
	{
		iter.into_iter().fold(Styles::new(), ops::BitOr::bitor)
	}
}

impl Extend<Style> for Styles {
	fn extend<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = Style>,
	{
		for style in iter {
			self.0 |= style as u32;
		}
	}
}

pub struct StylesIter {
	bits: u32,
}

impl StylesIter {
	const fn new(styles: Styles) -> Self {
		Self { bits: styles.0 }
	}
}

impl Iterator for StylesIter {
	type Item = Style;

	fn next(&mut self) -> Option<Self::Item> {
		while self.bits != 0 {
			let lsb = 1 << self.bits.trailing_zeros();
			self.bits &= !lsb;

			match Style::try_from(lsb) {
				Ok(style) => return Some(style),
				Err(_) => continue,
			}
		}

		None
	}
}

impl IntoIterator for Styles {
	type Item = Style;
	type IntoIter = StylesIter;

	fn into_iter(self) -> Self::IntoIter {
		StylesIter::new(self)
	}
}

impl IntoIterator for &Styles {
	type Item = Style;
	type IntoIter = StylesIter;

	fn into_iter(self) -> Self::IntoIter {
		StylesIter::new(*self)
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<Style> for rand::distributions::Standard {
	fn sample<R>(&self, _rng: &mut R) -> Style
	where
		R: rand::Rng + ?Sized,
	{
		// TODO: actually add randomness once we have more than one style
		Style::AutoBhop
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for Style {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Style {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::either::Either;

		Either::<u32, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(int) => int.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(u64::from(int)),
					&"a cs2kz style",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for Styles {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeSeq;

		let mut serializer = serializer.serialize_seq(None)?;

		for style in self {
			serializer.serialize_element(&style)?;
		}

		serializer.end()
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Styles {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		struct VisitStyles(Styles);

		impl<'de> de::Visitor<'de> for VisitStyles {
			type Value = Styles;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				write!(formatter, "a cs2kz style")
			}

			fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				while let Some(style) = seq.next_element::<Style>()? {
					self.0 |= style;
				}

				Ok(self.0)
			}
		}

		deserializer.deserialize_seq(VisitStyles(Styles::new()))
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for Style
where
	DB: sqlx::Database,
	u32: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<u32 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<u32 as sqlx::Type<DB>>::compatible(ty)
	}
}

#[cfg(feature = "sqlx")]
impl<'q, DB> sqlx::Encode<'q, DB> for Style
where
	DB: sqlx::Database,
	u32: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u32 as sqlx::Encode<'q, DB>>::encode_by_ref(&(*self as u32), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u32 as sqlx::Encode<'q, DB>>::encode(self as u32, buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<u32 as sqlx::Encode<'q, DB>>::produces(&(*self as u32))
	}

	fn size_hint(&self) -> usize {
		<u32 as sqlx::Encode<'q, DB>>::size_hint(&(*self as u32))
	}
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for Style
where
	DB: sqlx::Database,
	u32: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<u32 as sqlx::Decode<'r, DB>>::decode(value)
			.and_then(|value| value.try_into().map_err(Into::into))
	}
}
