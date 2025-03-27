/// Steam account instance bits
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, FromStr)]
pub struct AccountInstance(u32);

impl AccountInstance
{
	/// The default account instance
	pub const DEFAULT: Self = Self(1);

	/// Extracts the account instance bits from a raw 64-bit SteamID.
	pub const fn from_bits(bits: u64) -> Self
	{
		// TODO: what are the rules for this?
		Self(((bits << 12) >> 44) as u32)
	}

	/// Returns the raw integer value.
	pub const fn raw(self) -> u32
	{
		self.0
	}
}
