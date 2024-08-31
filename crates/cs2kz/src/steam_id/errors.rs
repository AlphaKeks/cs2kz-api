//! Error types used by the [`steam_id`] module.
//!
//! [`steam_id`]: crate::steam_id

use thiserror::Error;

/// An error indicating that a conversion to a SteamID failed because the source value was out of
/// range.
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
#[error("out of range for a valid SteamID")]
pub struct OutOfRange;

/// Errors returned by [`SteamID`]'s [`FromStr`] implementation.
///
/// [`SteamID`]: super::SteamID
/// [`FromStr`]: std::str::FromStr
#[non_exhaustive]
#[derive(Debug, PartialEq, Error)]
pub enum ParseSteamID
{
	/// The string was actually an integer but it was out of range.
	#[error(transparent)]
	OutOfRange(#[from] OutOfRange),

	/// The string did not match any known formats.
	#[error("unrecognized SteamID format")]
	UnrecognizedFormat,
}

/// Errors returned from [`SteamID::parse_standard()`].
///
/// [`SteamID::parse_standard()`]: super::SteamID::parse_standard
#[derive(Debug, PartialEq, Error)]
pub enum ParseStandardSteamIDError<'a>
{
	/// SteamIDs all start with `STEAM_`.
	#[error("missing `STEAM_ID` prefix")]
	MissingPrefix,

	/// The X segment in `STEAM_X:Y:Z` was missing.
	#[error("missing X segment")]
	MissingX,

	/// The X segment in `STEAM_X:Y:Z` was not 0 or 1.
	#[error("X segment should be 0 or 1 but is `{actual}`")]
	InvalidX
	{
		/// The actual value.
		actual: &'a str,
	},

	/// The Y segment in `STEAM_X:Y:Z` was missing.
	#[error("missing Y segment")]
	MissingY,

	/// The Y segment in `STEAM_X:Y:Z` was not 0 or 1.
	#[error("Y segment should be 0 or 1 but is `{actual}`")]
	InvalidY
	{
		/// The actual value.
		actual: &'a str,
	},

	/// The Z segment in `STEAM_X:Y:Z` was missing.
	#[error("missing Z segment")]
	MissingZ,

	/// The Z segment in `STEAM_X:Y:Z` was not a valid integer.
	#[error("invalid Z segment: `{actual}`")]
	InvalidZ
	{
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
///
/// [`SteamID::parse_community()`]: super::SteamID::parse_community
#[derive(Debug, PartialEq, Error)]
pub enum ParseCommunitySteamIDError<'a>
{
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
	InvalidID
	{
		/// The actual value.
		actual: &'a str,

		/// The source error we got from trying to parse the segment.
		source: std::num::ParseIntError,
	},

	/// The `XXXXXXXXX` segment in `U:1:XXXXXXXXX` was out of range for a valid SteamID.
	#[error("SteamID out of range")]
	OutOfRange,
}
