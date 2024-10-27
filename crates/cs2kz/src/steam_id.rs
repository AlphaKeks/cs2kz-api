//! [`SteamID`] and related types.

use std::borrow::{Borrow, Cow};
use std::num::{NonZero, ParseIntError};
use std::str::FromStr;
use std::{fmt, ops};

/// This is a sanity-check static assertion.
///
/// Because `SteamID` wraps a `NonZero` and is annotated with
/// `#[repr(transparent)]`, we should receive null-pointer-optimization.
const _ASSERT_NPO: () = assert!(
	size_of::<SteamID>() == size_of::<Option<SteamID>>(),
	"`SteamID` is marked `#[repr(transparent)]` and only contains a `NonZero<u64>`, so they \
	 should receive the same layout optimizations"
);

/// The minimum value for a valid SteamID.
const MIN: u64 = 76561197960265729_u64;

/// The minimum value for a valid SteamID.
const MAX: u64 = 76561202255233023_u64;

/// Used for bit operations, see implementation below.
const MAGIC_OFFSET: u64 = MIN - 1;

/// A type for working with [Valve's SteamID format][valve-docs].
///
/// [valve-docs]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZero<u64>);

/// Error saying that a conversion from an integer into a [`SteamID`] failed
/// because the integer was out of range.
#[non_exhaustive]
#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
#[error("value is out of range for a valid SteamID")]
pub struct OutOfRange;

/// Errors that can occur when parsing a string into a [`SteamID`] assuming the
/// standard format.
#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
#[expect(
	missing_docs,
	reason = "variant names + error messages should be self-documenting"
)]
pub enum ParseStandardError<'a> {
	#[error("missing `STEAM_` prefix")]
	MissingPrefix,

	#[error("missing `X` segment")]
	MissingX,

	#[error("invalid `X` segment; expected '0' or '1' but got `{segment}`")]
	InvalidX { segment: Cow<'a, str> },

	#[error("missing `Y` segment")]
	MissingY,

	#[error("invalid `Y` segment; expected '0' or '1' but got `{segment}`")]
	InvalidY { segment: Cow<'a, str> },

	#[error("missing `Z` segment")]
	MissingZ,

	#[error("invalid `Z` segment; expected u64 but got `{segment}`")]
	InvalidZ {
		segment: Cow<'a, str>,
		source: ParseIntError,
	},

	#[error("invalid `Z` segment; `{value}` is out of range for a valid SteamID")]
	OutOfRangeZ { value: u64 },

	#[error("SteamID cannot be all zeros")]
	IsZero,

	#[error("out of range for a valid SteamID")]
	OutOfRange,
}

impl ParseStandardError<'_> {
	/// Turns `self` into a `ParseStandardError<'static>` regardless of the
	/// current lifetime.
	fn into_static(self) -> ParseStandardError<'static> {
		match self {
			Self::MissingPrefix => ParseStandardError::MissingPrefix,
			Self::MissingX => ParseStandardError::MissingX,
			Self::InvalidX { segment } => ParseStandardError::InvalidX {
				segment: Cow::Owned(segment.into_owned()),
			},
			Self::MissingY => ParseStandardError::MissingY,
			Self::InvalidY { segment } => ParseStandardError::InvalidY {
				segment: Cow::Owned(segment.into_owned()),
			},
			Self::MissingZ => ParseStandardError::MissingZ,
			Self::InvalidZ { segment, source } => ParseStandardError::InvalidZ {
				segment: Cow::Owned(segment.into_owned()),
				source,
			},
			Self::OutOfRangeZ { value } => ParseStandardError::OutOfRangeZ { value },
			Self::IsZero => ParseStandardError::IsZero,
			Self::OutOfRange => ParseStandardError::OutOfRange,
		}
	}
}

/// Errors that can occur when parsing a string into a [`SteamID`] assuming the
/// "Steam3ID" format.
#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
#[expect(
	missing_docs,
	reason = "variant names + error messages should be self-documenting"
)]
pub enum ParseID3Error<'a> {
	#[error("valid Steam3ID must either have no brackets or both")]
	InconsistentBrackets,

	#[error("missing first segment")]
	MissingFirstSegment,

	#[error("invalid first segment; expected 'U' but got `{segment}`")]
	InvalidFirstSegment { segment: Cow<'a, str> },

	#[error("missing second segment")]
	MissingSecondSegment,

	#[error("invalid second segment; expected '1' but got `{segment}`")]
	InvalidSecondSegment { segment: Cow<'a, str> },

	#[error("missing third segment")]
	MissingThirdSegment,

	#[error("invalid third segment; expected u32 but got `{segment}`")]
	InvalidThirdSegment {
		segment: Cow<'a, str>,
		source: ParseIntError,
	},

	#[error("out of range for a valid SteamID")]
	OutOfRange,
}

impl ParseID3Error<'_> {
	/// Turns `self` into a `ParseID3Error<'static>` regardless of the current
	/// lifetime.
	fn into_static(self) -> ParseID3Error<'static> {
		match self {
			Self::InconsistentBrackets => ParseID3Error::InconsistentBrackets,
			Self::MissingFirstSegment => ParseID3Error::MissingFirstSegment,
			Self::InvalidFirstSegment { segment } => ParseID3Error::InvalidFirstSegment {
				segment: Cow::Owned(segment.into_owned()),
			},
			Self::MissingSecondSegment => ParseID3Error::MissingSecondSegment,
			Self::InvalidSecondSegment { segment } => ParseID3Error::InvalidSecondSegment {
				segment: Cow::Owned(segment.into_owned()),
			},
			Self::MissingThirdSegment => ParseID3Error::MissingThirdSegment,
			Self::InvalidThirdSegment { segment, source } => ParseID3Error::InvalidThirdSegment {
				segment: Cow::Owned(segment.into_owned()),
				source,
			},
			Self::OutOfRange => ParseID3Error::OutOfRange,
		}
	}
}

/// Errors that can occur when parsing a string into a [`SteamID`].
#[derive(Debug, Error)]
#[cfg_attr(test, derive(PartialEq))]
#[expect(
	missing_docs,
	reason = "variant names + error messages should be self-documenting"
)]
pub enum ParseError {
	#[error("out of range for a valid SteamID")]
	OutOfRange,

	#[error(transparent)]
	ParseStandard(#[from] ParseStandardError<'static>),

	#[error(transparent)]
	ParseID3(#[from] ParseID3Error<'static>),

	#[error("unrecognized SteamID format")]
	UnrecognizedFormat,
}

impl From<OutOfRange> for ParseError {
	fn from(_: OutOfRange) -> Self {
		Self::OutOfRange
	}
}

impl SteamID {
	/// The smallest valid [`SteamID`].
	pub const MIN: Self = match Self::new(MIN) {
		Some(steam_id) => steam_id,
		None => unreachable!(),
	};

	/// The smallest valid [`SteamID`].
	pub const MAX: Self = match Self::new(MAX) {
		Some(steam_id) => steam_id,
		None => unreachable!(),
	};

	/// Creates a new [`SteamID`] from a 64-bit value.
	///
	/// This function returns [`None`] if `value` is out of range. It assumes
	/// `value` is a 64-bit SteamID and does not make attempts to recognize
	/// other formats (such as a 32-bit ID disguised as a 64-bit integer).
	#[inline]
	pub const fn new(value: u64) -> Option<Self> {
		match value {
			// SAFETY: We make sure `value` is in range.
			MIN..=MAX => Some(unsafe { Self::new_unchecked(value) }),
			_ => None,
		}
	}

	/// Creates a new [`SteamID`] from a 64-bit value without checking that
	/// `value` is in range.
	///
	/// # Safety
	///
	/// The caller must guarantee that `value` is between [`SteamID::MIN`] and
	/// [`SteamID::MAX`] (inclusive on both ends).
	#[inline]
	#[track_caller]
	pub const unsafe fn new_unchecked(value: u64) -> Self {
		debug_assert!(
			MIN <= value && value <= MAX,
			"violated unsafe precondition: `value` is not a valid SteamID"
		);

		// SAFETY: The caller guarantees `value` is in range.
		Self(unsafe { NonZero::<u64>::new_unchecked(value) })
	}

	/// Creates a new [`SteamID`] from a 32-bit value.
	///
	/// This function returns [`None`] if `value` is out of range. It assumes
	/// `value` is a 32-bit SteamID.
	#[inline]
	pub const fn from_u32(value: u32) -> Option<Self> {
		Self::new((value as u64) + MAGIC_OFFSET)
	}

	/// Returns the SteamID as a 64-bit integer.
	#[inline]
	pub const fn as_u64(self) -> u64 {
		self.0.get()
	}

	/// Returns the SteamID as a 32-bit integer.
	#[inline]
	pub const fn as_u32(self) -> u32 {
		(((self.z() + self.y()) * 2) - self.y()) as u32
	}

	/// Returns the `X` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	#[inline]
	pub const fn x(&self) -> u64 {
		self.as_u64() >> 56
	}

	/// Returns the `Y` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	#[inline]
	pub const fn y(&self) -> u64 {
		self.as_u64() & 1
	}

	/// Returns the `Z` segment in `STEAM_X:Y:Z`.
	#[inline]
	pub const fn z(&self) -> u64 {
		(self.as_u64() - MAGIC_OFFSET - self.y()) / 2
	}

	/// Parses `value` assuming the standard SteamID format of `STEAM_X:Y:Z`.
	#[expect(clippy::many_single_char_names)]
	pub fn parse_standard(value: &str) -> Result<Self, ParseStandardError<'_>> {
		let mut segments = value
			.strip_prefix("STEAM_")
			.ok_or(ParseStandardError::MissingPrefix)?
			.splitn(3, ':');

		match segments.next() {
			Some("0" | "1") => {},
			Some(segment) => {
				return Err(ParseStandardError::InvalidX {
					segment: Cow::Borrowed(segment),
				});
			},
			None => return Err(ParseStandardError::MissingX),
		}

		let y = match segments.next() {
			Some("0") => 0,
			Some("1") => 1,
			Some(segment) => {
				return Err(ParseStandardError::InvalidY {
					segment: Cow::Borrowed(segment),
				});
			},
			None => return Err(ParseStandardError::MissingY),
		};

		let z = segments.next().ok_or(ParseStandardError::MissingZ)?;
		let z = z
			.parse::<u64>()
			.map_err(|source| ParseStandardError::InvalidZ {
				segment: Cow::Borrowed(z),
				source,
			})?;

		if y == 0 && z == 0 {
			return Err(ParseStandardError::IsZero);
		}

		if (z + MAGIC_OFFSET) > MAX {
			return Err(ParseStandardError::OutOfRangeZ { value: z });
		}

		Self::new(MAGIC_OFFSET | y | (z << 1)).ok_or(ParseStandardError::OutOfRange)
	}

	/// Parses `value` assuming the "Steam3ID" format of `U:1:XXXXXXXX`.
	pub fn parse_id3(value: &str) -> Result<Self, ParseID3Error<'_>> {
		#[expect(
			clippy::string_slice,
			reason = "we check that `value` starts and ends with ASCII characters, so slicing \
			          will not panic"
		)]
		let mut segments = match (value.starts_with('['), value.ends_with(']')) {
			(false, false) => value,
			(true, true) => &value[1..value.len() - 1],
			(true, false) | (false, true) => return Err(ParseID3Error::InconsistentBrackets),
		}
		.splitn(3, ':');

		match segments.next() {
			Some("U") => {},
			Some(segment) => {
				return Err(ParseID3Error::InvalidFirstSegment {
					segment: Cow::Borrowed(segment),
				});
			},
			None => return Err(ParseID3Error::MissingFirstSegment),
		}

		match segments.next() {
			Some("1") => {},
			Some(segment) => {
				return Err(ParseID3Error::InvalidSecondSegment {
					segment: Cow::Borrowed(segment),
				});
			},
			None => return Err(ParseID3Error::MissingSecondSegment),
		}

		let id32 = segments.next().ok_or(ParseID3Error::MissingThirdSegment)?;
		let id32 = id32
			.parse::<u32>()
			.map_err(|source| ParseID3Error::InvalidThirdSegment {
				segment: Cow::Borrowed(id32),
				source,
			})?;

		Self::from_u32(id32).ok_or(ParseID3Error::OutOfRange)
	}
}

impl fmt::Debug for SteamID {
	#[expect(clippy::many_single_char_names)]
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let x = self.x();
		let y = self.y();
		let z = self.z();

		if fmt.alternate() {
			fmt.debug_struct("SteamID")
				.field("X", &x)
				.field("Y", &y)
				.field("Z", &z)
				.finish()
		} else {
			write!(fmt, "STEAM_{x}:{y}:{z}")
		}
	}
}

impl fmt::Display for SteamID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		if fmt.alternate() {
			write!(fmt, "U:1:{}", self.as_u32())
		} else {
			write!(fmt, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
		}
	}
}

impl fmt::Binary for SteamID {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		<u64 as fmt::Binary>::fmt(&**self, fmt)
	}
}

impl ops::Deref for SteamID {
	type Target = u64;

	#[inline]
	fn deref(&self) -> &Self::Target {
		// SAFETY:
		//    1. Casting from `*const NonZero<u64>` to `*const u64` is sound because
		//       `NonZero<T>` is marked `#[repr(transparent)]`.
		//    2. Dereferencing that pointer is sound because it was created from a safe
		//       reference.
		//    3. We only ever expose shared references to the data stored inside
		//       `NonZero`, so we never break any of its invariants.
		unsafe { &*(&raw const self.0).cast::<u64>() }
	}
}

impl Borrow<u64> for SteamID {
	#[inline]
	fn borrow(&self) -> &u64 {
		&**self
	}
}

impl Borrow<NonZero<u64>> for SteamID {
	#[inline]
	fn borrow(&self) -> &NonZero<u64> {
		&self.0
	}
}

macro_rules! impl_partial_ops {
	($t1:ty: [$($t2:ty),* $(,)?]) => {
		$(impl PartialEq<$t2> for $t1
		{
			fn eq(&self, other: &$t2) -> bool
			{
				<$t2 as PartialEq<$t2>>::eq(self.borrow(), other)
			}
		}

		impl PartialEq<$t1> for $t2
		{
			fn eq(&self, other: &$t1) -> bool
			{
				<$t2 as PartialEq<$t2>>::eq(self, other.borrow())
			}
		}

		impl PartialOrd<$t2> for $t1
		{
			fn partial_cmp(&self, other: &$t2) -> Option<::std::cmp::Ordering>
			{
				<$t2 as PartialOrd<$t2>>::partial_cmp(self.borrow(), other)
			}
		}

		impl PartialOrd<$t1> for $t2
		{
			fn partial_cmp(&self, other: &$t1) -> Option<::std::cmp::Ordering>
			{
				<$t2 as PartialOrd<$t2>>::partial_cmp(self, other.borrow())
			}
		})*
	};
}

impl_partial_ops!(SteamID: [u64, NonZero<u64>]);

impl From<SteamID> for NonZero<u64> {
	fn from(SteamID(value): SteamID) -> Self {
		value
	}
}

impl From<SteamID> for u64 {
	fn from(SteamID(value): SteamID) -> Self {
		value.get()
	}
}

impl TryFrom<NonZero<u64>> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: NonZero<u64>) -> Result<Self, Self::Error> {
		Self::new(value.get()).ok_or(OutOfRange)
	}
}

impl TryFrom<u64> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		Self::new(value).ok_or(OutOfRange)
	}
}

impl TryFrom<u32> for SteamID {
	type Error = OutOfRange;

	fn try_from(value: u32) -> Result<Self, Self::Error> {
		Self::from_u32(value).ok_or(OutOfRange)
	}
}

impl FromStr for SteamID {
	type Err = ParseError;

	fn from_str(str: &str) -> Result<Self, Self::Err> {
		if let Ok(id64) = str.parse::<u64>() {
			return if let Ok(id32) = u32::try_from(id64) {
				Self::try_from(id32)
			} else {
				Self::try_from(id64)
			}
			.map_err(Into::into);
		}

		if str.starts_with("STEAM_") {
			return Self::parse_standard(str).map_err(|err| err.into_static().into());
		}

		if str.starts_with("U:") || str.starts_with("[U:") {
			return Self::parse_id3(str).map_err(|err| err.into_static().into());
		}

		Err(ParseError::UnrecognizedFormat)
	}
}

/// Various custom serialization functions.
///
/// You can use these in combination with the `#[serde(serialize_with = "…")]`
/// attribute when deriving [`serde::Serialize`].
///
/// The [`serde::Serialize`] implementation for [`SteamID`] forwards to
/// [`SteamID::serialize_standard()`].
#[cfg(feature = "serde")]
impl SteamID {
	/// Serializes this SteamID using the standard `STEAM_X:Y:Z` format.
	pub fn serialize_standard<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(&format_args!("{self}"), serializer)
	}

	/// Serializes this SteamID using the "Steam3ID" format.
	pub fn serialize_id3<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(&format_args!("{self:#}"), serializer)
	}

	/// Serializes this SteamID as a 64-bit integer.
	pub fn serialize_u64<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(&**self, serializer)
	}

	/// Serializes this SteamID as a stringified 64-bit integer.
	pub fn serialize_u64_stringified<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::Serialize::serialize(&format_args!("{}", &**self), serializer)
	}
}

/// By default [`SteamID`]s are serialized using the standard `STEAM_X:Y:Z`
/// format.
///
/// If you require a different format, you can use one of the various
/// `SteamID::serialize_*` methods in combination with the
/// `#[serde(serialize_with = "…")]` attribute.
#[cfg(feature = "serde")]
impl serde::Serialize for SteamID {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_standard(serializer)
	}
}

/// Various custom deserialization functions.
///
/// You can use these in combination with the `#[serde(deserialize_with = "…")]`
/// attribute when deriving [`serde::Deserialize`].
///
/// The [`serde::Deserialize`] implementation for [`SteamID`] takes a
/// best-effort approach to allow as many inputs as possible. If you have
/// stricter requirements or know your specific input format, consider using one
/// of the `SteamID::deserialize_*` functions instead.
#[cfg(feature = "serde")]
impl SteamID {
	/// Deserializes a string in the standard `STEAM_X:Y:Z` format into a
	/// [`SteamID`].
	pub fn deserialize_standard<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let input = <&'de str as serde::Deserialize<'de>>::deserialize(deserializer)?;

		Self::parse_standard(input).map_err(serde::de::Error::custom)
	}

	/// Deserializes a string in the "Steam3ID" format into a [`SteamID`].
	pub fn deserialize_id3<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let input = <&'de str as serde::Deserialize<'de>>::deserialize(deserializer)?;

		Self::parse_id3(input).map_err(serde::de::Error::custom)
	}

	/// Deserializes a u64 into a [`SteamID`].
	pub fn deserialize_u64<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let input = <u64 as serde::Deserialize<'de>>::deserialize(deserializer)?;

		Self::try_from(input).map_err(serde::de::Error::custom)
	}

	/// Deserializes a u32 into a [`SteamID`].
	pub fn deserialize_u32<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let input = <u32 as serde::Deserialize<'de>>::deserialize(deserializer)?;

		Self::try_from(input).map_err(serde::de::Error::custom)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for SteamID {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		#[derive(serde::Deserialize)]
		#[serde(untagged)]
		enum Helper<'a> {
			U32(u32),
			U64(u64),
			Str(&'a str),
		}

		<Helper<'de> as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			match value {
				Helper::U32(id32) => Self::try_from(id32).map_err(serde::de::Error::custom),
				Helper::U64(id64) => Self::try_from(id64).map_err(serde::de::Error::custom),
				Helper::Str(str) => str.parse::<Self>().map_err(serde::de::Error::custom),
			}
		})
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
		<u64 as sqlx::Encode<'q, DB>>::encode_by_ref(&**self, buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u64 as sqlx::Encode<'q, DB>>::encode(*self, buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<u64 as sqlx::Encode<'q, DB>>::produces(&**self)
	}

	fn size_hint(&self) -> usize {
		<u64 as sqlx::Encode<'q, DB>>::size_hint(&**self)
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
		<u64 as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map_err(Into::into)
	}
}

#[cfg(test)]
mod tests {
	use super::SteamID;

	const ALPHAKEKS: SteamID = match SteamID::new(76561198282622073_u64) {
		Some(steam_id) => steam_id,
		None => unreachable!(),
	};

	#[test]
	fn new() {
		assert!(SteamID::new(76561198282622073_u64).is_some());

		assert!(SteamID::new(76561197960265728_u64).is_none());
		assert!(SteamID::new(76561197960265729_u64).is_some());

		assert!(SteamID::new(76561202255233023_u64).is_some());
		assert!(SteamID::new(76561202255233024_u64).is_none());
	}

	#[test]
	fn try_from_u64() {
		assert!(SteamID::try_from(76561198282622073_u64).is_ok());

		assert!(SteamID::try_from(76561197960265728_u64).is_err());
		assert!(SteamID::try_from(76561197960265729_u64).is_ok());

		assert!(SteamID::try_from(76561202255233023_u64).is_ok());
		assert!(SteamID::try_from(76561202255233024_u64).is_err());
	}

	#[test]
	fn from_u32() {
		assert_eq!(SteamID::from_u32(322356345), Some(ALPHAKEKS));

		assert!(SteamID::from_u32(0).is_none());
		assert!(SteamID::from_u32(1).is_some());

		assert!(SteamID::from_u32((super::MAX - super::MAGIC_OFFSET) as u32).is_some());
		assert!(SteamID::from_u32((super::MAX - super::MAGIC_OFFSET + 1) as u32).is_none());
	}

	#[test]
	fn try_from_u32() {
		assert_eq!(SteamID::try_from(322356345_u32), Ok(ALPHAKEKS));

		assert!(SteamID::try_from(0_u32).is_err());
		assert!(SteamID::try_from(1_u32).is_ok());

		assert!(SteamID::try_from((super::MAX - super::MAGIC_OFFSET) as u32).is_ok());
		assert!(SteamID::try_from((super::MAX - super::MAGIC_OFFSET + 1) as u32).is_err());
	}

	#[test]
	fn parse_u64() {
		assert!("76561198282622073".parse::<SteamID>().is_ok());

		assert!("76561197960265728".parse::<SteamID>().is_err());
		assert!("76561197960265729".parse::<SteamID>().is_ok());

		assert!("76561202255233023".parse::<SteamID>().is_ok());
		assert!("76561202255233024_u64".parse::<SteamID>().is_err());
	}

	#[test]
	fn parse_u32() {
		assert_eq!("322356345".parse::<SteamID>(), Ok(ALPHAKEKS));

		assert!("0".parse::<SteamID>().is_err());
		assert!("1".parse::<SteamID>().is_ok());

		assert!(
			((super::MAX - super::MAGIC_OFFSET) as u32)
				.to_string()
				.parse::<SteamID>()
				.is_ok()
		);

		assert!(
			((super::MAX - super::MAGIC_OFFSET + 1) as u32)
				.to_string()
				.parse::<SteamID>()
				.is_err()
		);
	}

	#[test]
	fn parse_standard() {
		assert_eq!("STEAM_0:1:161178172".parse::<SteamID>(), Ok(ALPHAKEKS));
		assert_eq!("STEAM_1:1:161178172".parse::<SteamID>(), Ok(ALPHAKEKS));
	}

	#[test]
	fn parse_id3() {
		assert_eq!("U:1:322356345".parse::<SteamID>(), Ok(ALPHAKEKS));
		assert_eq!("[U:1:322356345]".parse::<SteamID>(), Ok(ALPHAKEKS));

		assert!("U:1:322356345]".parse::<SteamID>().is_err());
		assert!("[U:1:322356345".parse::<SteamID>().is_err());
	}
}
