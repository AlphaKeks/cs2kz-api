//! A transparent wrapper around [`u64`] for working with [SteamID]s.
//!
//! [SteamID]: https://developer.valvesoftware.com/wiki/SteamID

use std::borrow::Borrow;
use std::num::NonZero;
use std::str::FromStr;
use std::{fmt, ops, ptr};

mod errors;
pub use errors::{OutOfRange, ParseCommunitySteamIDError, ParseStandardSteamIDError, ParseSteamID};

cfg_rand! {
	mod rand;
}

cfg_serde! {
	mod serde;
}

cfg_sqlx! {
	mod sqlx;
}

#[allow(clippy::missing_docs_in_private_items, clippy::missing_assert_message)]
const _: () = assert!(size_of::<SteamID>() == size_of::<Option<SteamID>>());

/// The minimum value for a valid SteamID.
const MIN: u64 = 76561197960265729_u64;

/// The minimum value for a valid SteamID.
const MAX: u64 = 76561202255233023_u64;

/// Used for bit operations, see implementation below.
const MAGIC_OFFSET: u64 = MIN - 1;

/// A [SteamID].
///
/// [SteamID]: https://developer.valvesoftware.com/wiki/SteamID
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SteamID(NonZero<u64>);

impl SteamID
{
	/// The minimum value for a valid [`SteamID`].
	pub const MIN: Self = unsafe { Self::from_u64_unchecked(MIN) };

	/// The maximum value for a valid [`SteamID`].
	pub const MAX: Self = unsafe { Self::from_u64_unchecked(MAX) };

	/// Returns the `X` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn x(&self) -> u64
	{
		let x = self.0.get() >> 56;
		debug_assert_matches!(x, 0 | 1, "SteamID X segment has an invalid value");
		x
	}

	/// Returns the `Y` segment in `STEAM_X:Y:Z`.
	///
	/// This will always be 0 or 1.
	pub const fn y(&self) -> u64
	{
		let y = self.0.get() & 1;
		debug_assert_matches!(y, 0 | 1, "SteamID Y segment has an invalid value");
		y
	}

	/// Returns the `Z` segment in `STEAM_X:Y:Z`.
	pub const fn z(&self) -> u64
	{
		(self.0.get() - MAGIC_OFFSET - self.y()) / 2
	}

	/// Returns the `SteamID` in its 64-bit representation.
	pub const fn as_u64(&self) -> u64
	{
		self.0.get()
	}

	/// Returns the `SteamID` in its 32-bit representation.
	pub const fn as_u32(&self) -> u32
	{
		let value = ((self.z() + self.y()) * 2) - self.y();

		debug_assert!(
			0 < value && value <= (u32::MAX as u64),
			"SteamID 32-bit representation has an invalid value",
		);

		value as u32
	}

	/// Creates a new [`SteamID`] from a 64-bit integer.
	///
	/// This assumes the bits are arranged as described in [the valve documentation].
	/// If `value` is out of range, [`None`] will be returned.
	///
	/// [the valve documentation]: https://developer.valvesoftware.com/wiki/SteamID#As_Represented_in_Computer_Programs
	pub const fn from_u64(value: u64) -> Option<Self>
	{
		if matches!(value, MIN..=MAX) {
			// SAFETY: we checked that `value` is in-bounds
			Some(unsafe { Self::from_u64_unchecked(value) })
		} else {
			None
		}
	}

	/// Creates a new [`SteamID`] from a 64-bit integer without performing bounds checks on
	/// `value`.
	///
	/// # Safety
	///
	/// The caller must ensure that `value` is within `SteamID::MIN..=SteamID::MAX`.
	pub const unsafe fn from_u64_unchecked(value: u64) -> Self
	{
		debug_assert_matches!(value, MIN..=MAX, "out of range SteamID in unsafe function");

		// SAFETY: the caller must uphold the invariants
		Self(unsafe { NonZero::new_unchecked(value) })
	}

	/// Creates a new [`SteamID`] from a 32-bit integer.
	///
	/// If `value` is out of range, [`None`] will be returned.
	pub const fn from_u32(value: u32) -> Option<Self>
	{
		Self::from_u64((value as u64) + MAGIC_OFFSET)
	}

	/// Creates a new [`SteamID`] from a 32-bit integer without performing bounds checks on
	/// `value`.
	///
	/// # Safety
	///
	/// The caller must ensure that `value` is in-range.
	pub const unsafe fn from_u32_unchecked(value: u32) -> Self
	{
		// SAFETY: the caller must uphold the invariants
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
	pub fn parse_standard(mut value: &str) -> Result<Self, ParseStandardSteamIDError<'_>>
	{
		#[allow(clippy::missing_docs_in_private_items)]
		const PREFIX: &str = "STEAM_";

		if value.starts_with(PREFIX) {
			value = &value[const { PREFIX.len() }..];
		} else {
			return Err(ParseStandardSteamIDError::MissingPrefix);
		}

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

		Self::from_u64(MAGIC_OFFSET | y | (z << 1)).ok_or(ParseStandardSteamIDError::OutOfRange)
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
	pub fn parse_community(mut value: &str) -> Result<Self, ParseCommunitySteamIDError<'_>>
	{
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

		Self::from_u32(id32).ok_or(ParseCommunitySteamIDError::OutOfRange)
	}
}

impl fmt::Debug for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		if f.alternate() {
			f.debug_struct("SteamID")
				.field("X", &self.x())
				.field("Y", &self.y())
				.field("Z", &self.z())
				.finish()
		} else {
			fmt::Display::fmt(self, f)
		}
	}
}

impl fmt::Display for SteamID
{
	/// Formats a [`SteamID`] in a human-readable format.
	///
	/// By default this uses the `STEAM_X:Y:Z` format, as described in
	/// [the valve documentation]. The `#` sigil can be used to format as a
	/// ["Steam Community ID"].
	///
	/// # Examples
	///
	/// ```
	/// use cs2kz::SteamID;
	///
	/// let steam_id = SteamID::MIN;
	///
	/// assert_eq!(format!("{steam_id}"), "STEAM_1:1:0");
	/// assert_eq!(format!("{steam_id:#}"), "U:1:1");
	/// ```
	///
	/// [the valve documentation]: https://developer.valvesoftware.com/wiki/SteamID#As_Represented_Textually
	/// ["Steam Community ID"]: https://developer.valvesoftware.com/wiki/SteamID#Steam_ID_as_a_Steam_Community_ID
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		if f.alternate() {
			write!(f, "U:1:{}", self.as_u32())
		} else {
			write!(f, "STEAM_{}:{}:{}", self.x(), self.y(), self.z())
		}
	}
}

impl fmt::Binary for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Binary::fmt(&self.0, f)
	}
}

impl fmt::LowerHex for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::LowerHex::fmt(&self.0, f)
	}
}

impl fmt::UpperHex for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::UpperHex::fmt(&self.0, f)
	}
}

impl fmt::Octal for SteamID
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Octal::fmt(&self.0, f)
	}
}

impl Borrow<u64> for SteamID
{
	fn borrow(&self) -> &u64
	{
		// SAFETY:
		//    1. we are marked `#[repr(transparent)]`, which means we have the same layout
		//       as `NonZero<u64>`
		//    2. `NonZero<T>` is marked `#[repr(transparent)]`, which means it has the same
		//       layout as the type it's wrapping (`u64` in this case)
		//    3. we never give out a mutable reference to the inner value
		unsafe { &*ptr::addr_of!(self.0).cast::<u64>() }
	}
}

impl Borrow<NonZero<u64>> for SteamID
{
	fn borrow(&self) -> &NonZero<u64>
	{
		&self.0
	}
}

impl AsRef<u64> for SteamID
{
	fn as_ref(&self) -> &u64
	{
		self.borrow()
	}
}

impl AsRef<NonZero<u64>> for SteamID
{
	fn as_ref(&self) -> &NonZero<u64>
	{
		self.borrow()
	}
}

impl ops::Deref for SteamID
{
	type Target = u64;

	fn deref(&self) -> &Self::Target
	{
		self.borrow()
	}
}

#[allow(clippy::missing_docs_in_private_items)]
macro_rules! impl_partial_ops {
	($t1:ty => [$($t2:ty),* $(,)?]) => {
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

impl_partial_ops!(SteamID => [u64, NonZero<u64>]);

impl From<SteamID> for u64
{
	fn from(steam_id: SteamID) -> Self
	{
		steam_id.0.get()
	}
}

impl From<SteamID> for NonZero<u64>
{
	fn from(steam_id: SteamID) -> Self
	{
		steam_id.0
	}
}

impl TryFrom<u64> for SteamID
{
	type Error = OutOfRange;

	fn try_from(value: u64) -> Result<Self, Self::Error>
	{
		if let Ok(value) = u32::try_from(value) {
			Self::from_u32(value)
		} else {
			Self::from_u64(value)
		}
		.ok_or(OutOfRange)
	}
}

impl TryFrom<NonZero<u64>> for SteamID
{
	type Error = OutOfRange;

	fn try_from(value: NonZero<u64>) -> Result<Self, Self::Error>
	{
		Self::try_from(value.get())
	}
}

impl TryFrom<u32> for SteamID
{
	type Error = OutOfRange;

	fn try_from(value: u32) -> Result<Self, Self::Error>
	{
		Self::from_u32(value).ok_or(OutOfRange)
	}
}

impl FromStr for SteamID
{
	type Err = ParseSteamID;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
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

#[cfg(test)]
mod tests
{
	use super::*;

	const ALPHAKEKS_RAW: u64 = 76561198282622073_u64;
	const ALPHAKEKS: SteamID = steam_id!(76561198282622073);

	#[test]
	fn x_works()
	{
		assert_eq!(ALPHAKEKS.x(), 1_u64);
	}

	#[test]
	fn y_works()
	{
		assert_eq!(ALPHAKEKS.y(), 1_u64);
	}

	#[test]
	fn z_works()
	{
		assert_eq!(ALPHAKEKS.z(), 161178172_u64);
	}

	#[test]
	fn as_u32_works()
	{
		assert_eq!(ALPHAKEKS.as_u32(), 322356345_u32);
	}

	#[test]
	fn from_u64_works()
	{
		assert_matches!(SteamID::from_u64(ALPHAKEKS_RAW), Some(ALPHAKEKS));
	}

	#[test]
	fn from_u32_works()
	{
		assert_matches!(SteamID::from_u32(322356345_u32), Some(ALPHAKEKS));
	}

	#[test]
	fn parse_standard_works()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_0:1:161178172"),
			Ok(ALPHAKEKS),
		);

		assert_matches!(
			SteamID::parse_standard("STEAM_1:1:161178172"),
			Ok(ALPHAKEKS)
		);
	}

	#[test]
	fn parse_standard_fails_if_prefix_is_missing()
	{
		assert_matches!(
			SteamID::parse_standard("0:1:161178172"),
			Err(ParseStandardSteamIDError::MissingPrefix),
		);
	}

	#[test]
	fn parse_standard_fails_if_x_is_missing()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_:1:161178172"),
			Err(ParseStandardSteamIDError::MissingX),
		);
	}

	#[test]
	fn parse_standard_fails_if_x_is_invalid()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_2:1:161178172"),
			Err(ParseStandardSteamIDError::InvalidX { actual: "2" }),
		);
	}

	#[test]
	fn parse_standard_fails_if_y_is_missing()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_1:"),
			Err(ParseStandardSteamIDError::MissingY),
		);
	}

	#[test]
	fn parse_standard_fails_if_y_is_invalid()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_1:3:161178172"),
			Err(ParseStandardSteamIDError::InvalidY { actual: "3" }),
		);
	}

	#[test]
	fn parse_standard_fails_if_z_is_missing()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_1:0:"),
			Err(ParseStandardSteamIDError::MissingZ),
		);
	}

	#[test]
	fn parse_standard_fails_if_z_is_invalid()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_1:0:foobar"),
			Err(ParseStandardSteamIDError::InvalidZ {
				actual: "foobar",
				..
			}),
		);
	}

	#[test]
	fn parse_standard_fails_if_zero()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_0:0:0"),
			Err(ParseStandardSteamIDError::IsZero),
		);
	}

	#[test]
	fn parse_standard_fails_if_out_of_range()
	{
		assert_matches!(
			SteamID::parse_standard("STEAM_1:0:9999999999"),
			Err(ParseStandardSteamIDError::OutOfRange),
		);
	}
}
