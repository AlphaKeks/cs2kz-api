use {crate::InvalidAccountUniverse, std::num::ParseIntError};

/// Error type for conversions from [`u64`] to [`SteamId`]
///
/// [`SteamId`]: crate::SteamId
#[allow(missing_copy_implementations)]
#[derive(Debug, Display, Error)]
#[display("invalid SteamID64: {_variant}")]
pub enum InvalidSteamId64
{
	/// The universe bits were invalid.
	#[display("invalid universe bits")]
	InvalidUniverse,

	/// The account type bits were invalid.
	#[display("invalid account type bits")]
	InvalidAccountType,
}

/// Error type for conversions from strings to [`SteamId`]
///
/// [`SteamId`]: crate::SteamId
#[derive(Debug, Display, Error, From)]
#[display("failed to parse SteamID: {_variant}")]
pub enum ParseSteamIdError
{
	/// The format could not be detected.
	#[display("unknown format")]
	UnknownFormat,

	/// The format was determined to be a 64-bit integer, but it was invalid.
	InvalidSteamId64(InvalidSteamId64),
}

/// Error type for parsing Steam2ID strings
#[derive(Debug, Display, Error, From)]
#[display("failed to parse Steam2ID: {_variant}")]
pub enum ParseSteam2IdError
{
	#[display("missing `STEAM_` prefix")]
	MissingPrefix,

	#[display("missing `X` segment")]
	MissingX,

	#[display("invalid `X` segment: {_0}")]
	InvalidX(InvalidAccountUniverse),

	#[display("missing `Y` segment")]
	MissingY,

	#[display("invalid `Y` segment")]
	InvalidY,

	#[display("missing `Z` segment")]
	MissingZ,

	#[display("invalid `Z` segment: {_0}")]
	InvalidZ(ParseIntError),
}
