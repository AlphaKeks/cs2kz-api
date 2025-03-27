/// Steam account number bits
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromStr)]
pub struct AccountNumber(u32);

impl AccountNumber
{
	/// Extracts the account number bits from a raw 64-bit SteamID.
	pub const fn from_bits(bits: u64) -> Self
	{
		// TODO: what are the rules for this?
		Self(((bits << 32) >> 33) as u32)
	}

	/// Returns the raw integer value.
	pub const fn raw(self) -> u32
	{
		self.0
	}
}
