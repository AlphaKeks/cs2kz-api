use std::str::FromStr;
use std::{fmt, ops};

/// User permissions.
#[repr(u64)]
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Permission {
	/// The user can (un)ban players.
	#[display("bans")]
	Bans = 1 << 0,

	/// The user can update records.
	#[display("records")]
	Records = 1 << 1,

	/// The user can approve and manage CS2 servers.
	#[display("servers")]
	Servers = 1 << 8,

	/// The user can approve and manage global maps.
	#[display("maps")]
	Maps = 1 << 16,

	/// The user can manage other users' permissions.
	#[display("admin")]
	Admin = 1 << 31,
}

/// 0 or more [`Permission`]s combined as bitflags.
#[derive(
	Display, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into, Deref, sqlx::Type,
)]
#[display("{self:?}")]
#[sqlx(transparent)]
pub struct Permissions(u64);

#[derive(Debug, Error)]
#[error("unknown permission")]
pub struct UnknownPermission;

impl Permissions {
	pub fn all() -> Self {
		Permission::Bans
			| Permission::Records
			| Permission::Servers
			| Permission::Maps
			| Permission::Admin
	}

	/// Checks `self` is empty, i.e. contains no permissions.
	pub fn is_empty(&self) -> bool {
		self.0 == 0
	}

	/// Checks if the given `permissions` is in `self`.
	pub fn contains(self, permissions: Permissions) -> bool {
		(self.0 & permissions.0) == permissions.0
	}

	/// Returns an iterator over the [`Permission`]s in this set.
	pub fn iter(&self) -> PermissionsIter {
		PermissionsIter::new(*self)
	}
}

impl fmt::Debug for Permissions {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut iter = self.into_iter();

		write!(f, "[")?;

		if let Some(permission) = iter.next() {
			write!(f, "{permission}")?;
		}

		for permission in iter {
			write!(f, ", {permission}")?;
		}

		write!(f, "]")
	}
}

impl ops::BitAnd for Permissions {
	type Output = Permissions;

	fn bitand(self, rhs: Permissions) -> Self::Output {
		Permissions(self.0 & rhs.0)
	}
}

impl ops::BitAnd<Permission> for Permissions {
	type Output = Permissions;

	fn bitand(self, rhs: Permission) -> Self::Output {
		Permissions(self.0 & (rhs as u64))
	}
}

impl ops::BitAnd<Permissions> for Permission {
	type Output = Permissions;

	fn bitand(self, rhs: Permissions) -> Self::Output {
		Permissions(rhs.0 & (self as u64))
	}
}

impl ops::BitAndAssign for Permissions {
	fn bitand_assign(&mut self, rhs: Permissions) {
		self.0 &= rhs.0;
	}
}

impl ops::BitAndAssign<Permission> for Permissions {
	fn bitand_assign(&mut self, rhs: Permission) {
		self.0 &= rhs as u64;
	}
}

impl ops::BitOr for Permissions {
	type Output = Permissions;

	fn bitor(self, rhs: Permissions) -> Self::Output {
		Permissions(self.0 | rhs.0)
	}
}

impl ops::BitOr<Permission> for Permissions {
	type Output = Permissions;

	fn bitor(self, rhs: Permission) -> Self::Output {
		Permissions(self.0 | (rhs as u64))
	}
}

impl ops::BitOr for Permission {
	type Output = Permissions;

	fn bitor(self, rhs: Permission) -> Self::Output {
		Permissions((rhs as u64) | (self as u64))
	}
}

impl ops::BitOr<Permissions> for Permission {
	type Output = Permissions;

	fn bitor(self, rhs: Permissions) -> Self::Output {
		Permissions(rhs.0 | (self as u64))
	}
}

impl ops::BitOrAssign for Permissions {
	fn bitor_assign(&mut self, rhs: Permissions) {
		self.0 |= rhs.0;
	}
}

impl ops::BitOrAssign<Permission> for Permissions {
	fn bitor_assign(&mut self, rhs: Permission) {
		self.0 |= rhs as u64;
	}
}

impl ops::BitXor for Permissions {
	type Output = Permissions;

	fn bitxor(self, rhs: Permissions) -> Self::Output {
		Permissions(self.0 ^ rhs.0)
	}
}

impl ops::BitXor<Permission> for Permissions {
	type Output = Permissions;

	fn bitxor(self, rhs: Permission) -> Self::Output {
		Permissions(self.0 ^ (rhs as u64))
	}
}

impl ops::BitXor<Permissions> for Permission {
	type Output = Permissions;

	fn bitxor(self, rhs: Permissions) -> Self::Output {
		Permissions(rhs.0 ^ (self as u64))
	}
}

impl ops::BitXorAssign for Permissions {
	fn bitxor_assign(&mut self, rhs: Permissions) {
		self.0 ^= rhs.0;
	}
}

impl ops::BitXorAssign<Permission> for Permissions {
	fn bitxor_assign(&mut self, rhs: Permission) {
		self.0 ^= rhs as u64;
	}
}

impl From<Permission> for u64 {
	fn from(permission: Permission) -> Self {
		permission as u64
	}
}

impl From<Permission> for Permissions {
	fn from(permission: Permission) -> Self {
		Permissions(permission as u64)
	}
}

impl TryFrom<u64> for Permission {
	type Error = UnknownPermission;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		const BANS: u64 = Permission::Bans as u64;
		const RECORDS: u64 = Permission::Records as u64;
		const SERVERS: u64 = Permission::Servers as u64;
		const MAPS: u64 = Permission::Maps as u64;
		const ADMIN: u64 = Permission::Admin as u64;

		match value {
			BANS => Ok(Permission::Bans),
			RECORDS => Ok(Permission::Records),
			SERVERS => Ok(Permission::Servers),
			MAPS => Ok(Permission::Maps),
			ADMIN => Ok(Permission::Admin),
			_ => Err(UnknownPermission),
		}
	}
}

impl FromStr for Permission {
	type Err = UnknownPermission;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(value) = value.parse::<u64>() {
			return Self::try_from(value);
		}

		match value {
			"bans" => Ok(Permission::Bans),
			"records" => Ok(Permission::Records),
			"servers" => Ok(Permission::Servers),
			"maps" => Ok(Permission::Maps),
			"admin" => Ok(Permission::Admin),
			_ => Err(UnknownPermission),
		}
	}
}

impl FromIterator<Permission> for Permissions {
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Permission>,
	{
		iter.into_iter()
			.fold(Permissions::default(), ops::BitOr::bitor)
	}
}

impl Extend<Permission> for Permissions {
	fn extend<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = Permission>,
	{
		for permission in iter {
			self.0 |= permission as u64;
		}
	}
}

pub struct PermissionsIter {
	bits: u64,
}

impl PermissionsIter {
	fn new(permissions: Permissions) -> Self {
		Self {
			bits: permissions.0,
		}
	}
}

impl Iterator for PermissionsIter {
	type Item = Permission;

	fn next(&mut self) -> Option<Self::Item> {
		while self.bits != 0 {
			let lsb = 1 << self.bits.trailing_zeros();
			self.bits &= !lsb;

			match Permission::try_from(lsb) {
				Ok(permission) => return Some(permission),
				Err(_) => continue,
			}
		}

		None
	}
}

impl IntoIterator for Permissions {
	type Item = Permission;
	type IntoIter = PermissionsIter;

	fn into_iter(self) -> Self::IntoIter {
		PermissionsIter::new(self)
	}
}

impl IntoIterator for &Permissions {
	type Item = Permission;
	type IntoIter = PermissionsIter;

	fn into_iter(self) -> Self::IntoIter {
		PermissionsIter::new(*self)
	}
}

impl serde::Serialize for Permission {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Permission {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::serde::Either;

		Either::<u64, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(int) => int.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(int),
					&"a user permission",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

impl serde::Serialize for Permissions {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		use serde::ser::SerializeSeq;

		let mut serializer = serializer.serialize_seq(None)?;

		for permission in self {
			serializer.serialize_element(&permission)?;
		}

		serializer.end()
	}
}

impl<'de> serde::Deserialize<'de> for Permissions {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		struct VisitPermissions(Permissions);

		impl<'de> de::Visitor<'de> for VisitPermissions {
			type Value = Permissions;

			fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
				write!(formatter, "a user permission")
			}

			fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				while let Some(permission) = seq.next_element::<Permission>()? {
					self.0 |= permission;
				}

				Ok(self.0)
			}
		}

		deserializer.deserialize_seq(VisitPermissions(Permissions::default()))
	}
}

mod utoipa_impls {
	use utoipa::openapi::{self, ObjectBuilder, RefOr, Schema};
	use utoipa::{PartialSchema, ToSchema};

	use super::*;

	impl PartialSchema for Permissions {
		fn schema() -> RefOr<Schema> {
			Schema::Array(
				ObjectBuilder::new()
					.schema_type(openapi::Type::String)
					.enum_values(Some(["bans", "records", "servers", "maps", "admin"]))
					.to_array_builder()
					.build(),
			)
			.into()
		}
	}

	impl ToSchema for Permissions {}
}
