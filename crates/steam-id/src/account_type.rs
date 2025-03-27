use std::str::FromStr;

/// Different types of Steam accounts
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType
{
	Invalid = 0,
	Individual = 1,
	Multiseat = 2,
	GameServer = 3,
	AnonGameServer = 4,
	Pending = 5,
	ContentServer = 6,
	Clan = 7,
	Chat = 8,
	P2P = 9,
	AnonUser = 10,
}

/// Error type for conversions from strings to [`AccountType`]
#[allow(missing_copy_implementations)]
#[derive(Debug, Display, Error)]
#[display("invalid account type")]
pub struct InvalidAccountType(#[error(ignore)] ());

impl AccountType
{
	/// Extracts the account type bits from a raw 64-bit SteamID.
	///
	/// If the bits are invalid, this function will return [`None`].
	pub const fn from_bits(bits: u64) -> Option<Self>
	{
		match (bits << 8) >> 60 {
			0 => Some(Self::Invalid),
			1 => Some(Self::Individual),
			2 => Some(Self::Multiseat),
			3 => Some(Self::GameServer),
			4 => Some(Self::AnonGameServer),
			5 => Some(Self::Pending),
			6 => Some(Self::ContentServer),
			7 => Some(Self::Clan),
			8 => Some(Self::Chat),
			9 => Some(Self::P2P),
			10 => Some(Self::AnonUser),
			_ => None,
		}
	}
}

impl FromStr for AccountType
{
	type Err = InvalidAccountType;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value {
			"I" | "i" | "invalid" | "Invalid" => Ok(Self::Invalid),
			"U" | "individual" | "Individual" => Ok(Self::Individual),
			"M" | "multiseat" | "Multiseat" => Ok(Self::Multiseat),
			"G" | "gameserver" | "GameServer" => Ok(Self::GameServer),
			"A" | "anongameserver" | "AnonGameServer" => Ok(Self::AnonGameServer),
			"P" | "pending" | "Pending" => Ok(Self::Pending),
			"C" | "contentserver" | "ContentServer" => Ok(Self::ContentServer),
			"g" | "clan" | "Clan" => Ok(Self::Clan),
			"T" | "L" | "c" => Ok(Self::Chat),
			"a" | "anonuser" | "AnonUser" => Ok(Self::AnonUser),
			_ => Err(InvalidAccountType(())),
		}
	}
}
