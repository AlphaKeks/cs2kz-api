use std::{fmt, ops};

const BANS: u64 = 1 << 0;
const RECORDS: u64 = 1 << 1;
const SERVERS: u64 = 1 << 8;
const MAPS: u64 = 1 << 16;
const ADMIN: u64 = 1 << 31;

/// A permission.
#[repr(u64)]
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Permission {
	/// The user can manage player bans.
	Bans = BANS,

	/// The user can manage records.
	Records = RECORDS,

	/// The user can manage CS2 servers.
	Servers = SERVERS,

	/// The user can manage approved maps.
	Maps = MAPS,

	/// The user can manage other users' permissions.
	Admin = ADMIN,
}

/// A set of [`Permission`]s.
#[derive(Default, Clone, Copy, PartialEq, Eq, sqlx::Type, utoipa::ToSchema)]
#[sqlx(transparent)]
#[schema(value_type = Vec<Permission>)]
pub struct Permissions(u64);

/// An iterator over [`Permission`]s.
#[derive(Debug)]
pub struct PermissionsIter {
	bits: u64,
}

#[derive(Debug, Error)]
pub enum TryFromIntError {
	#[error("value contains multiple bits")]
	MultipleBits,

	#[error("set bit does not belong to any permission")]
	InvalidBit,
}

impl Permissions {
	/// Returns whether there are no permission bits set in `self`.
	pub fn is_empty(&self) -> bool {
		self.0 == 0
	}

	/// Checks whether `other` is a subset of `self`.
	pub fn contains(self, other: Self) -> bool {
		(self.0 & other.0) == other.0
	}

	/// Checks whether `permission` is present in `self`.
	pub fn contains_permission(self, permission: Permission) -> bool {
		(self.0 & (permission as u64)) == (permission as u64)
	}

	/// Returns an iterator over the [`Permission`]s stored in `self`.
	pub fn iter(&self) -> PermissionsIter {
		self.into_iter()
	}
}

impl fmt::Debug for Permissions {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		struct DebugInner<'a>(&'a Permissions);

		impl fmt::Debug for DebugInner<'_> {
			fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
				fmt.debug_list().entries(self.0).finish()
			}
		}

		fmt.debug_tuple("Permissions")
			.field(&DebugInner(self))
			.finish()
	}
}

impl ops::BitOr for Permission {
	type Output = Permissions;

	fn bitor(self, rhs: Self) -> Self::Output {
		Permissions(self as u64 | rhs as u64)
	}
}

impl ops::BitOr<Permissions> for Permission {
	type Output = Permissions;

	fn bitor(self, rhs: Permissions) -> Self::Output {
		Permissions(self as u64 | rhs.0)
	}
}

impl ops::BitOr for Permissions {
	type Output = Self;

	fn bitor(self, rhs: Self) -> Self::Output {
		Self(self.0 | rhs.0)
	}
}

impl ops::BitOr<Permission> for Permissions {
	type Output = Self;

	fn bitor(self, rhs: Permission) -> Self::Output {
		Self(self.0 | rhs as u64)
	}
}

impl ops::BitOrAssign for Permissions {
	fn bitor_assign(&mut self, rhs: Self) {
		self.0 |= rhs.0;
	}
}

impl ops::BitOrAssign<Permission> for Permissions {
	fn bitor_assign(&mut self, rhs: Permission) {
		self.0 |= rhs as u64;
	}
}

impl From<Permission> for Permissions {
	fn from(permission: Permission) -> Self {
		Self(permission as u64)
	}
}

impl TryFrom<u64> for Permission {
	type Error = TryFromIntError;

	fn try_from(value: u64) -> Result<Self, Self::Error> {
		match value {
			BANS => Ok(Self::Bans),
			RECORDS => Ok(Self::Records),
			SERVERS => Ok(Self::Servers),
			MAPS => Ok(Self::Maps),
			ADMIN => Ok(Self::Admin),
			_ if value.count_ones() > 1 => Err(TryFromIntError::MultipleBits),
			_ => Err(TryFromIntError::InvalidBit),
		}
	}
}

impl IntoIterator for &Permissions {
	type Item = Permission;
	type IntoIter = PermissionsIter;

	fn into_iter(self) -> Self::IntoIter {
		PermissionsIter { bits: self.0 }
	}
}

impl IntoIterator for Permissions {
	type Item = Permission;
	type IntoIter = PermissionsIter;

	fn into_iter(self) -> Self::IntoIter {
		PermissionsIter { bits: self.0 }
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

		deserializer.deserialize_seq(VisitPermissions(Self::default()))
	}
}

impl Iterator for PermissionsIter {
	type Item = Permission;

	fn next(&mut self) -> Option<Self::Item> {
		if self.bits == 0 {
			return None;
		}

		let lsb = 1 << self.bits.trailing_zeros();
		self.bits &= !lsb;

		Some(
			Permission::try_from(lsb)
				.expect("`Permissions` should contain only known permission bits"),
		)
	}
}

mod utoipa_impls {
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, schema};
	use utoipa::{PartialSchema, ToSchema};

	use super::Permission;

	impl PartialSchema for Permission {
		fn schema() -> RefOr<Schema> {
			Schema::Object(
				ObjectBuilder::new()
					.schema_type(schema::Type::String)
					.title(Some("Permission"))
					.description(Some("a user permission"))
					.enum_values(Some(["bans", "records", "servers", "maps", "admin"]))
					.build(),
			)
			.into()
		}
	}

	impl ToSchema for Permission {}
}
