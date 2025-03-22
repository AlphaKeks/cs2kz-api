use std::str::FromStr;

/// Steam account universes
///
/// See: <https://developer.valvesoftware.com/wiki/SteamID#Universes_Available_for_Steam_Accounts>
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountUniverse
{
	Individual = 0,
	Public = 1,
	Beta = 2,
	Internal = 3,
	Dev = 4,
	RC = 5,
}

/// Error type for conversions from strings to [`AccountUniverse`]
#[allow(missing_copy_implementations)]
#[derive(Debug, Display, Error)]
#[display("invalid account universe")]
pub struct InvalidAccountUniverse(#[error(ignore)] ());

impl AccountUniverse
{
	/// Extracts the universe bits from a raw 64-bit SteamID.
	///
	/// If the bits are invalid, this function will return [`None`].
	pub const fn from_bits(bits: u64) -> Option<Self>
	{
		match bits >> 56 {
			0 => Some(Self::Individual),
			1 => Some(Self::Public),
			2 => Some(Self::Beta),
			3 => Some(Self::Internal),
			4 => Some(Self::Dev),
			5 => Some(Self::RC),
			_ => None,
		}
	}
}

impl FromStr for AccountUniverse
{
	type Err = InvalidAccountUniverse;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"individual" | "Individual" | "unspecified" | "Unspecified" => Ok(Self::Individual),
			"public" | "Public" => Ok(Self::Public),
			"beta" | "Beta" => Ok(Self::Beta),
			"internal" | "Internal" => Ok(Self::Internal),
			"dev" | "Dev" => Ok(Self::Dev),
			"RC" => Ok(Self::RC),
			_ => Err(InvalidAccountUniverse(())),
		}
	}
}
