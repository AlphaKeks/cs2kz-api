use std::borrow::Borrow;
use std::num::NonZero;
use std::str::FromStr;
use std::{cmp, fmt, ops, ptr};

// `SteamID` wraps a `NonZero<u64>` specifically for the layout optimizations, so we want to ensure
// statically we actually get those optmiziations.
const _: () = assert!(size_of::<SteamID>() == size_of::<Option<SteamID>>());

const MIN: u64 = 76561197960265729_u64;
const MAX: u64 = 76561202255233023_u64;
const MAGIC_OFFSET: u64 = MIN - 1;

/// A SteamID.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZero<u64>);

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("value is out of range for a SteamID")]
pub struct OutOfRange;

/// Errors returned from [`SteamID::parse_standard()`].
#[derive(Debug, Error)]
pub enum ParseStandardSteamIDError<'a> {
	/// SteamIDs all start with `STEAM_`.
	#[error("missing `STEAM_ID` prefix")]
	MissingPrefix,

	/// The X segment in `STEAM_X:Y:Z` was missing.
	#[error("missing X segment")]
	MissingX,

	/// The X segment in `STEAM_X:Y:Z` was not 0 or 1.
	#[error("X segment should be 0 or 1 but is `{actual}`")]
	InvalidX {
		/// The actual value.
		actual: &'a str,
	},

	/// The Y segment in `STEAM_X:Y:Z` was missing.
	#[error("missing Y segment")]
	MissingY,

	/// The Y segment in `STEAM_X:Y:Z` was not 0 or 1.
	#[error("Y segment should be 0 or 1 but is `{actual}`")]
	InvalidY {
		/// The actual value.
		actual: &'a str,
	},

	/// The Z segment in `STEAM_X:Y:Z` was missing.
	#[error("missing Z segment")]
	MissingZ,

	/// The Z segment in `STEAM_X:Y:Z` was not a valid integer.
	#[error("invalid Z segment: `{actual}`")]
	InvalidZ {
		/// The actual value.
		actual: &'a str,

		/// The source error we got from trying to parse the segment.
		source: std::num::ParseIntError,
	},

	/// The resulting SteamID would be 0, which is out of range.
	#[error("SteamID is 0")]
	IsZero,

	/// The resulting SteamID would be out of range.
	#[error("SteamID is out of range")]
	OutOfRange,
}

/// Errors returned from [`SteamID::parse_community()`].
#[derive(Debug, Error)]
pub enum ParseCommunitySteamIDError<'a> {
	/// Only one of the `[]` brackets around the SteamID was present.
	#[error("inconsistent brackets surrounding SteamID")]
	InconsistentBrackets,

	/// The account type segment (`U`) in `U:1:XXXXXXXXX` was missing.
	#[error("missing account type segment")]
	MissingAccountType,

	/// The `1` segment in `U:1:XXXXXXXXX` was missing.
	#[error("missing `1` segment")]
	MissingOne,

	/// The `XXXXXXXXX` segment in `U:1:XXXXXXXXX` was missing.
	#[error("missing ID segment")]
	MissingID,

	/// The `XXXXXXXXX` segment in `U:1:XXXXXXXXX` was not a valid 32-bit integer.
	#[error("invalid ID segment")]
	InvalidID {
		/// The actual value.
		actual: &'a str,

		/// The source error we got from trying to parse the segment.
		source: std::num::ParseIntError,
	},

	/// The `XXXXXXXXX` segment in `U:1:XXXXXXXXX` was out of range for a valid SteamID.
	#[error("SteamID out of range")]
	OutOfRange,
}

/// Errors returned by [`SteamID`]'s [`FromStr`] implementation.
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum ParseSteamID {
	/// The string was actually an integer but it was out of range.
	#[error(transparent)]
	OutOfRange(#[from] OutOfRange),

	/// The string did not match any known formats.
	#[error("unrecognized SteamID format")]
	UnrecognizedFormat,
}

impl SteamID {
	/// The lowest valid SteamID.
	pub const MIN: Self = match NonZero::new(MIN) {
		Some(v) => Self(v),
		None => unreachable!(),
	};

	/// The highest valid SteamID.
	pub const MAX: Self = match NonZero::new(MAX) {
		Some(v) => Self(v),
		None => unreachable!(),
	};

	/// Returns the `X` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn x(&self) -> u64 {
		self.0.get() >> 56
	}

	/// Returns the `Y` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn y(&self) -> u64 {
		self.0.get() & 1
	}

	/// Returns the `Z` segment in `STEAM_X:Y:Z`.
	pub const fn z(&self) -> u64 {
		(self.0.get() - MAGIC_OFFSET - self.y()) / 2
	}

	/// Returns the `SteamID` in its 64-bit representation.
	pub const fn as_u64(&self) -> u64 {
		self.0.get()
	}

	/// Returns the `SteamID` in its 32-bit representation.
	pub const fn as_u32(&self) -> u32 {
		(((self.z() + self.y()) * 2) - self.y()) as u32
	}

	/// Creates a new [`SteamID`] from its 64-bit representation.
	///
	/// This function will fail if `value` is not between [`MIN`] and [`MAX`] (inclusive).
	///
	/// [`MIN`]: SteamID::MIN
	/// [`MAX`]: SteamID::MAX
	pub const fn from_u64(value: u64) -> Result<Self, OutOfRange> {
		if matches!(value, MIN..=MAX) {
			// SAFETY: we performed the necessary bounds check
			Ok(unsafe { Self::from_u64_unchecked(value) })
		} else {
			Err(OutOfRange)
		}
	}

	/// Creates a new [`SteamID`] from its 64-bit representation, without performing bounds checks.
	///
	/// # Safety
	///
	/// The caller must guarantee that `value` is between [`MIN`] and [`MAX`] (inclusive).
	///
	/// [`MIN`]: SteamID::MIN
	/// [`MAX`]: SteamID::MAX
	pub const unsafe fn from_u64_unchecked(value: u64) -> Self {
		debug_assert!(matches!(value, MIN..=MAX), "out-of-range SteamID");

		// SAFETY: the caller must guarantee that `value` is not 0
		Self(unsafe { NonZero::new_unchecked(value) })
	}

	/// Creates a new [`SteamID`] from its 32-bit representation.
	///
	/// This function will fail if `value` is out of range.
	pub const fn from_u32(value: u32) -> Result<Self, OutOfRange> {
		Self::from_u64((value as u64) + MAGIC_OFFSET)
	}

	/// Creates a new [`SteamID`] from its 32-bit representation, without performing bounds checks.
	///
	/// # Safety
	///
	/// The caller must guarantee that `value` is in-range.
	pub const unsafe fn from_u32_unchecked(value: u32) -> Self {
		// SAFETY: the caller must guarantee that `value` is in-range
		unsafe { Self::from_u64_unchecked((value as u64) + MAGIC_OFFSET) }
	}

	/// Parses a [`SteamID`] in the standard format of `STEAM_X:Y:Z`.
	///
	/// # Examples
	///
	/// ```
	/// use cs2kz::SteamID;
	///
	/// let steam_id = SteamID::parse_standard("STEAM_1:1:161178172");
	///
	/// assert!(steam_id.is_ok());
	/// ```
	pub fn parse_standard(value: &str) -> Result<Self, ParseStandardSteamIDError<'_>> {
		let Some(("STEAM", value)) = value.split_once('_') else {
			return Err(ParseStandardSteamIDError::MissingPrefix);
		};

		let mut segments = value.splitn(3, ':');

		let _x = match segments.next() {
			// CS2 always uses 1
			Some("0" | "1") => 1,
			Some("") | None => return Err(ParseStandardSteamIDError::MissingX),
			Some(actual) => return Err(ParseStandardSteamIDError::InvalidX { actual }),
		};

		let y = match segments.next() {
			Some("0") => 0,
			Some("1") => 1,
			Some("") | None => return Err(ParseStandardSteamIDError::MissingY),
			Some(actual) => return Err(ParseStandardSteamIDError::InvalidY { actual }),
		};

		let z = segments
			.next()
			.filter(|s| !s.is_empty())
			.ok_or(ParseStandardSteamIDError::MissingZ)?;

		let z = z
			.parse::<u64>()
			.map_err(|source| ParseStandardSteamIDError::InvalidZ { actual: z, source })?;

		if y == 0 && z == 0 {
			return Err(ParseStandardSteamIDError::IsZero);
		}

		if (z + MAGIC_OFFSET) > MAX {
			return Err(ParseStandardSteamIDError::OutOfRange);
		}

		Self::from_u64(MAGIC_OFFSET | y | (z << 1))
			.map_err(|_| ParseStandardSteamIDError::OutOfRange)
	}

	/// Parses a [`SteamID`] in the Steam Community ID format of `U:1:XXXXXXXXX`, optionally
	/// enclosed in `[]`.
	///
	/// # Examples
	///
	/// ```
	/// use cs2kz::SteamID;
	///
	/// let steam_id = SteamID::parse_community("U:1:322356345");
	///
	/// assert!(steam_id.is_ok());
	/// ```
	pub fn parse_community(mut value: &str) -> Result<Self, ParseCommunitySteamIDError<'_>> {
		value = match (value.starts_with('['), value.ends_with(']')) {
			(false, false) => value,
			(true, true) => &value[1..value.len() - 2],
			(true, false) | (false, true) => {
				return Err(ParseCommunitySteamIDError::InconsistentBrackets);
			}
		};

		let mut segments = value.splitn(3, ':');

		if !matches!(segments.next(), Some("U")) {
			return Err(ParseCommunitySteamIDError::MissingAccountType);
		}

		if !matches!(segments.next(), Some("1")) {
			return Err(ParseCommunitySteamIDError::MissingOne);
		}

		let id32 = segments
			.next()
			.ok_or(ParseCommunitySteamIDError::MissingID)?;

		let id32 = id32
			.parse::<u32>()
			.map_err(|source| ParseCommunitySteamIDError::InvalidID {
				actual: id32,
				source,
			})?;

		Self::from_u32(id32).map_err(|_| ParseCommunitySteamIDError::OutOfRange)
	}
}

impl fmt::Debug for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if f.alternate() {
			return f
				.debug_struct("SteamID")
				.field("X", &self.x())
				.field("Y", &self.y())
				.field("Z", &self.z())
				.finish();
		}

		write!(f, "\"{self}\"")
	}
}

impl fmt::Display for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if f.alternate() {
			write!(f, "U:1:{}", self.as_u32())
		} else {
			write!(f, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
		}
	}
}

impl fmt::Binary for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for SteamID {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Octal::fmt(&self.0, f)
	}
}

impl Borrow<u64> for SteamID {
	fn borrow(&self) -> &u64 {
		// SAFETY:
		//    1. we are marked `#[repr(transparent)]`, which means we have the same layout
		//       as `NonZero<u64>`
		//    2. `NonZero<T>` is marked `#[repr(transparent)]`, which means it has the same
		//       layout as the type it's wrapping (`u64` in this case)
		//    3. we never give out a mutable reference to the inner value
		unsafe { &*ptr::addr_of!(self.0).cast::<u64>() }
	}
}

impl Borrow<NonZero<u64>> for SteamID {
	fn borrow(&self) -> &NonZero<u64> {
		&self.0
	}
}

impl AsRef<u64> for SteamID {
	fn as_ref(&self) -> &u64 {
		self.borrow()
	}
}

impl AsRef<NonZero<u64>> for SteamID {
	fn as_ref(&self) -> &NonZero<u64> {
		self.borrow()
	}
}

impl ops::Deref for SteamID {
	type Target = u64;

	fn deref(&self) -> &Self::Target {
		self.borrow()
	}
}

impl PartialEq<u64> for SteamID {
	fn eq(&self, other: &u64) -> bool {
		<u64 as PartialEq>::eq(other, self.borrow())
	}
}

impl PartialEq<SteamID> for u64 {
	fn eq(&self, other: &SteamID) -> bool {
		<u64 as PartialEq>::eq(self, other.borrow())
	}
}

impl PartialEq<NonZero<u64>> for SteamID {
	fn eq(&self, other: &NonZero<u64>) -> bool {
		<NonZero<u64> as PartialEq>::eq(other, self.borrow())
	}
}

impl PartialEq<SteamID> for NonZero<u64> {
	fn eq(&self, other: &SteamID) -> bool {
		<NonZero<u64> as PartialEq>::eq(self, other.borrow())
	}
}

impl PartialOrd<u64> for SteamID {
	fn partial_cmp(&self, other: &u64) -> Option<cmp::Ordering> {
		<u64 as PartialOrd>::partial_cmp(other, self.borrow())
	}
}

impl PartialOrd<SteamID> for u64 {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		<u64 as PartialOrd>::partial_cmp(self, other.borrow())
	}
}

impl PartialOrd<NonZero<u64>> for SteamID {
	fn partial_cmp(&self, other: &NonZero<u64>) -> Option<cmp::Ordering> {
		<NonZero<u64> as PartialOrd>::partial_cmp(other, self.borrow())
	}
}

impl PartialOrd<SteamID> for NonZero<u64> {
	fn partial_cmp(&self, other: &SteamID) -> Option<cmp::Ordering> {
		<NonZero<u64> as PartialOrd>::partial_cmp(self, other.borrow())
	}
}

impl From<SteamID> for u64 {
	fn from(steam_id: SteamID) -> Self {
		steam_id.as_u64()
	}
}

impl TryFrom<u64> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		Self::from_u64(value)
	}
}

impl From<SteamID> for NonZero<u64> {
	fn from(steam_id: SteamID) -> Self {
		steam_id.0
	}
}

impl TryFrom<NonZero<u64>> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: NonZero<u64>) -> Result<Self, Self::Error> {
		Self::from_u64(value.get())
	}
}

impl From<SteamID> for u32 {
	fn from(steam_id: SteamID) -> Self {
		steam_id.as_u32()
	}
}

impl TryFrom<u32> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		Self::from_u32(value)
	}
}

impl FromStr for SteamID {
	type Err = ParseSteamID;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(value) = value.parse::<u32>() {
			return Self::try_from(value).map_err(Into::into);
		}

		if let Ok(value) = value.parse::<u64>() {
			return Self::try_from(value).map_err(Into::into);
		}

		if let Ok(value) = Self::parse_standard(value) {
			return Ok(value);
		}

		if let Ok(value) = Self::parse_community(value) {
			return Ok(value);
		}

		Err(ParseSteamID::UnrecognizedFormat)
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<SteamID> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> SteamID
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(MIN..=MAX)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

/// By default [`SteamID`] uses [`SteamID::serialize_standard()`] for serialization.
///
/// If you require a different format, use the `#[serde(serialize_with = "…")]` attribute with one
/// of the `SteamID::serialize_*` functions.
#[cfg(feature = "serde")]
impl serde::Serialize for SteamID {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_standard(serializer)
	}
}

/// Various serialization functions for [`serde`].
///
/// You can use these with the `#[serde(serialize_with = "…")]` attribute to override the default
/// serialization format.
#[cfg(feature = "serde")]
impl SteamID {
	pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<u64 as serde::Serialize>::serialize(self.borrow(), serializer)
	}

	pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<fmt::Arguments<'_> as serde::Serialize>::serialize(
			&format_args!("{}", self.as_u64()),
			serializer,
		)
	}

	pub fn serialize_u32<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<u32 as serde::Serialize>::serialize(&self.as_u32(), serializer)
	}

	pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<fmt::Arguments<'_> as serde::Serialize>::serialize(&format_args!("{self}"), serializer)
	}

	pub fn serialize_community<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<fmt::Arguments<'_> as serde::Serialize>::serialize(&format_args!("{self:#}"), serializer)
	}
}

/// The default [`serde::Deserialize`] implementation is a best-effort attempt to deserialize most
/// inputs. If you know your format ahead of time, you might want to use the
/// `#[serde(deserialize_with = "…")]` attribute with one of the `SteamID::deserialize_*` functions
/// instead.
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SteamID {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::either::Either;

		type Int = Either<u32, u64>;

		Either::<Int, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(Either::A(int32)) => Self::try_from(int32).map_err(|_| {
				de::Error::invalid_value(
					de::Unexpected::Unsigned(u64::from(int32)),
					&"a 32-bit SteamID",
				)
			}),
			Either::A(Either::B(int64)) => Self::try_from(int64).map_err(|_| {
				de::Error::invalid_value(de::Unexpected::Unsigned(int64), &"a 64-bit SteamID")
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl SteamID {
	pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u64 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(value),
					&"a 64-bit SteamID",
				)
			})
		})
	}

	pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u32 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(u64::from(value)),
					&"a 32-bit SteamID",
				)
			})
		})
	}

	pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer)
			.and_then(|value| Self::parse_standard(value).map_err(serde::de::Error::custom))
	}

	pub fn deserialize_community<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer)
			.and_then(|value| Self::parse_community(value).map_err(serde::de::Error::custom))
	}

	pub fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer)
			.and_then(|value| value.parse::<Self>().map_err(serde::de::Error::custom))
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for SteamID
where
	DB: sqlx::Database,
	u64: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<u64 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<u64 as sqlx::Type<DB>>::compatible(ty)
	}
}

#[cfg(feature = "sqlx")]
impl<'q, DB> sqlx::Encode<'q, DB> for SteamID
where
	DB: sqlx::Database,
	u64: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(self.borrow(), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u64 as sqlx::Encode<'q, DB>>::encode(self.as_u64(), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<u64 as sqlx::Encode<'q, DB>>::produces(self.borrow())
	}

	fn size_hint(&self) -> usize {
		<u64 as sqlx::Encode<'q, DB>>::size_hint(self.borrow())
	}
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for SteamID
where
	DB: sqlx::Database,
	u64: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<u64 as sqlx::Decode<'r, DB>>::decode(value)
			.and_then(|value| value.try_into().map_err(Into::into))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const ALPHAKEKS_RAW: u64 = 76561198282622073_u64;
	const ALPHAKEKS: SteamID = match SteamID::from_u64(76561198282622073) {
		Ok(steam_id) => steam_id,
		Err(_) => unreachable!(),
	};

	#[test]
	fn x_works() {
		assert_eq!(ALPHAKEKS.x(), 1_u64);
	}

	#[test]
	fn y_works() {
		assert_eq!(ALPHAKEKS.y(), 1_u64);
	}

	#[test]
	fn z_works() {
		assert_eq!(ALPHAKEKS.z(), 161178172_u64);
	}

	#[test]
	fn as_u32_works() {
		assert_eq!(ALPHAKEKS.as_u32(), 322356345_u32);
	}

	#[test]
	fn from_u64_works() {
		assert!(matches!(SteamID::from_u64(ALPHAKEKS_RAW), Ok(ALPHAKEKS)));
	}

	#[test]
	fn from_u32_works() {
		assert!(matches!(SteamID::from_u32(322356345_u32), Ok(ALPHAKEKS)));
	}

	#[test]
	fn parse_standard_works() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_0:1:161178172"),
			Ok(ALPHAKEKS),
		));

		assert!(matches!(
			SteamID::parse_standard("STEAM_1:1:161178172"),
			Ok(ALPHAKEKS)
		));
	}

	#[test]
	fn parse_standard_fails_if_prefix_is_missing() {
		assert!(matches!(
			SteamID::parse_standard("0:1:161178172"),
			Err(ParseStandardSteamIDError::MissingPrefix),
		));
	}

	#[test]
	fn parse_standard_fails_if_x_is_missing() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_:1:161178172"),
			Err(ParseStandardSteamIDError::MissingX),
		));
	}

	#[test]
	fn parse_standard_fails_if_x_is_invalid() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_2:1:161178172"),
			Err(ParseStandardSteamIDError::InvalidX { actual: "2" }),
		));
	}

	#[test]
	fn parse_standard_fails_if_y_is_missing() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_1:"),
			Err(ParseStandardSteamIDError::MissingY),
		));
	}

	#[test]
	fn parse_standard_fails_if_y_is_invalid() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_1:3:161178172"),
			Err(ParseStandardSteamIDError::InvalidY { actual: "3" }),
		));
	}

	#[test]
	fn parse_standard_fails_if_z_is_missing() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_1:0:"),
			Err(ParseStandardSteamIDError::MissingZ),
		));
	}

	#[test]
	fn parse_standard_fails_if_z_is_invalid() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_1:0:foobar"),
			Err(ParseStandardSteamIDError::InvalidZ {
				actual: "foobar",
				..
			}),
		));
	}

	#[test]
	fn parse_standard_fails_if_zero() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_0:0:0"),
			Err(ParseStandardSteamIDError::IsZero),
		));
	}

	#[test]
	fn parse_standard_fails_if_out_of_range() {
		assert!(matches!(
			SteamID::parse_standard("STEAM_1:0:9999999999"),
			Err(ParseStandardSteamIDError::OutOfRange),
		));
	}
}
