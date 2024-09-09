#[derive(
	Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, sqlx::Type, serde::Serialize,
)]
#[sqlx(transparent)]
pub struct Permissions(u64);

impl Permissions
{
	pub const NONE: Self = Self(0);
	pub const MANAGE_SERVERS: Self = Self(1 << 1);
	pub const MANAGE_MAPS: Self = Self(1 << 2);
	pub const MANAGE_BANS: Self = Self(1 << 3);
	pub const MANAGE_RECORDS: Self = Self(1 << 4);
	pub const ADMIN: Self = Self(1 << 63);

	pub const ALL: Self = Self(
		Self::MANAGE_SERVERS.0 | Self::MANAGE_MAPS.0 | Self::MANAGE_BANS.0 | Self::MANAGE_RECORDS.0,
	);

	pub fn contains(self, other: Self) -> bool
	{
		self.0 & other.0 == other.0
	}
}
