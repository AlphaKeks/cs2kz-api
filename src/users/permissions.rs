use {
	serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser::SerializeSeq},
	std::{fmt, ops, str::FromStr},
	utoipa::ToSchema,
	zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, try_transmute_ref},
};

const CREATE_MAPS: u64 = 1_u64 << 0;
const UPDATE_MAPS: u64 = 1_u64 << 1;

const MODIFY_SERVER_METADATA: u64 = 1_u64 << 8;
const MODIFY_SERVER_BUDGETS: u64 = 1_u64 << 9;
const RESET_SERVER_ACCESS_KEYS: u64 = 1_u64 << 10;
const DELETE_SERVER_ACCESS_KEYS: u64 = 1_u64 << 11;

const CREATE_BANS: u64 = 1_u64 << 16;
const UPDATE_BANS: u64 = 1_u64 << 17;
const REVERT_BANS: u64 = 1_u64 << 18;

const GRANT_CREATE_MAPS: u64 = 1_u64 << 62;
const MODIFY_USER_PERMISSIONS: u64 = 1_u64 << 63;

#[repr(u64)]
#[non_exhaustive]
#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	TryFromBytes,
	IntoBytes,
	Immutable,
	KnownLayout,
	Serialize,
	Deserialize,
	ToSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Permission
{
	/// The user may create new KZ maps and submit them to the API.
	CreateMaps = CREATE_MAPS,

	/// The user may update existing KZ maps.
	///
	/// This includes changes to the map's metadata, such as the list of
	/// mappers, as well as updating its state (without the restrictions mappers
	/// have themselves).
	UpdateMaps = UPDATE_MAPS,

	/// The user may modify the metadata of all global servers.
	ModifyServerMetadata = MODIFY_SERVER_METADATA,

	/// The user may modify the server budgets of other users.
	ModifyServerBudgets = MODIFY_SERVER_BUDGETS,

	/// The user may reset the access keys of global servers.
	ResetServerAccessKeys = RESET_SERVER_ACCESS_KEYS,

	/// The user may delete the access keys of global servers.
	DeleteServerAccessKeys = DELETE_SERVER_ACCESS_KEYS,

	/// The user may ban players.
	CreateBans = CREATE_BANS,

	/// The user may update player bans.
	UpdateBans = UPDATE_BANS,

	/// The user may unban players.
	RevertBans = REVERT_BANS,

	/// The user may grant the [`CreateMaps`] permission to other users.
	///
	/// [`CreateMaps`]: Permission::CreateMaps
	GrantCreateMaps = GRANT_CREATE_MAPS,

	/// The user may modify other users' permissions.
	ModifyUserPermissions = MODIFY_USER_PERMISSIONS,
}

/// A set of [`Permission`]s
#[derive(
	Default,
	Clone,
	Copy,
	PartialEq,
	Eq,
	Hash,
	TryFromBytes,
	IntoBytes,
	Immutable,
	KnownLayout,
	sqlx::Type,
	ToSchema,
)]
#[sqlx(transparent)]
#[schema(value_type = [Permission])]
pub struct Permissions(u64);

#[derive(Debug, Display, Error)]
#[display("invalid permission: {reason}")]
pub struct InvalidPermission
{
	reason: Box<str>,
}

/// An [`Iterator`] over [`Permission`]s
#[derive(Debug, Clone)]
pub struct Iter
{
	bits: u64,
}

impl Permissions
{
	/// Returns the number of [`Permission`]s stored in `self`.
	pub fn count(&self) -> u32
	{
		self.0.count_ones()
	}

	/// Checks if `other` is a subset of `self`.
	pub fn contains<P>(&self, other: &P) -> bool
	where
		P: ?Sized + AsRef<Permissions>,
	{
		let other = other.as_ref();
		(self.0 & other.0) == other.0
	}

	/// Returns an [`Iterator`] over the [`Permission`]s stored in `self`.
	pub fn iter(&self) -> Iter
	{
		Iter { bits: self.0 }
	}
}

impl AsRef<Permissions> for Permission
{
	fn as_ref(&self) -> &Permissions
	{
		try_transmute_ref!(self).unwrap_or_else(|err| {
			panic!("conversions from `Permission` to `Permissions` should always succeed\n{err}");
		})
	}
}

impl ops::BitAnd for Permission
{
	type Output = Permissions;

	fn bitand(self, rhs: Permission) -> Self::Output
	{
		Permissions((self as u64) & (rhs as u64))
	}
}

impl ops::BitAnd<Permissions> for Permission
{
	type Output = Permissions;

	fn bitand(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self as u64) & (rhs.0))
	}
}

impl ops::BitOr for Permission
{
	type Output = Permissions;

	fn bitor(self, rhs: Permission) -> Self::Output
	{
		Permissions((self as u64) | (rhs as u64))
	}
}

impl ops::BitOr<Permissions> for Permission
{
	type Output = Permissions;

	fn bitor(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self as u64) | (rhs.0))
	}
}

impl ops::BitXor for Permission
{
	type Output = Permissions;

	fn bitxor(self, rhs: Permission) -> Self::Output
	{
		Permissions((self as u64) ^ (rhs as u64))
	}
}

impl FromStr for Permission
{
	type Err = InvalidPermission;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::deserialize(serde::de::value::StrDeserializer::new(value))
	}
}

impl ops::BitXor<Permissions> for Permission
{
	type Output = Permissions;

	fn bitxor(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self as u64) ^ (rhs.0))
	}
}

impl fmt::Debug for Permissions
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_set().entries(self).finish()
	}
}

impl From<Permission> for Permissions
{
	fn from(permission: Permission) -> Self
	{
		Self(permission as u64)
	}
}

impl IntoIterator for &Permissions
{
	type Item = Permission;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		self.iter()
	}
}

impl IntoIterator for Permissions
{
	type Item = Permission;
	type IntoIter = Iter;

	fn into_iter(self) -> Self::IntoIter
	{
		self.iter()
	}
}

impl Serialize for Permissions
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut serializer = serializer.serialize_seq(Some(self.count() as usize))?;

		for permission in self {
			serializer.serialize_element(&permission)?;
		}

		serializer.end()
	}
}

impl<'de> Deserialize<'de> for Permissions
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct PermissionsVisitor;

		impl<'de> de::Visitor<'de> for PermissionsVisitor
		{
			type Value = Permissions;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
			{
				fmt.write_str("a list of user permissions")
			}

			fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
			where
				A: de::SeqAccess<'de>,
			{
				let mut permissions = Permissions::default();

				while let Some(permission) = seq.next_element::<Permission>()? {
					permissions |= permission;
				}

				Ok(permissions)
			}
		}

		deserializer.deserialize_seq(PermissionsVisitor)
	}
}

impl ops::BitAnd for Permissions
{
	type Output = Permissions;

	fn bitand(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self.0) & (rhs.0))
	}
}

impl ops::BitAnd<Permission> for Permissions
{
	type Output = Permissions;

	fn bitand(self, rhs: Permission) -> Self::Output
	{
		Permissions((self.0) & (rhs as u64))
	}
}

impl ops::BitAndAssign for Permissions
{
	fn bitand_assign(&mut self, rhs: Permissions)
	{
		self.0 &= rhs.0;
	}
}

impl ops::BitAndAssign<Permission> for Permissions
{
	fn bitand_assign(&mut self, rhs: Permission)
	{
		self.0 &= rhs as u64;
	}
}

impl ops::BitOr for Permissions
{
	type Output = Permissions;

	fn bitor(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self.0) | (rhs.0))
	}
}

impl ops::BitOr<Permission> for Permissions
{
	type Output = Permissions;

	fn bitor(self, rhs: Permission) -> Self::Output
	{
		Permissions((self.0) | (rhs as u64))
	}
}

impl ops::BitOrAssign for Permissions
{
	fn bitor_assign(&mut self, rhs: Permissions)
	{
		self.0 |= rhs.0;
	}
}

impl ops::BitOrAssign<Permission> for Permissions
{
	fn bitor_assign(&mut self, rhs: Permission)
	{
		self.0 |= rhs as u64;
	}
}

impl ops::BitXor for Permissions
{
	type Output = Permissions;

	fn bitxor(self, rhs: Permissions) -> Self::Output
	{
		Permissions((self.0) ^ (rhs.0))
	}
}

impl ops::BitXor<Permission> for Permissions
{
	type Output = Permissions;

	fn bitxor(self, rhs: Permission) -> Self::Output
	{
		Permissions((self.0) ^ (rhs as u64))
	}
}

impl ops::BitXorAssign for Permissions
{
	fn bitxor_assign(&mut self, rhs: Permissions)
	{
		self.0 ^= rhs.0;
	}
}

impl ops::BitXorAssign<Permission> for Permissions
{
	fn bitxor_assign(&mut self, rhs: Permission)
	{
		self.0 ^= rhs as u64;
	}
}

impl FromIterator<Permission> for Permissions
{
	fn from_iter<I>(iter: I) -> Self
	where
		I: IntoIterator<Item = Permission>,
	{
		iter.into_iter().fold(Self::default(), ops::BitOr::bitor)
	}
}

impl serde::de::Error for InvalidPermission
{
	fn custom<T>(msg: T) -> Self
	where
		T: fmt::Display,
	{
		Self { reason: msg.to_string().into_boxed_str() }
	}
}

impl Iterator for Iter
{
	type Item = Permission;

	fn next(&mut self) -> Option<Self::Item>
	{
		if self.bits == 0 {
			return None;
		}

		let next_bit = 1 << self.bits.trailing_zeros();
		self.bits &= !next_bit;

		Permission::try_read_from_bytes(next_bit.as_bytes())
			.map_or_else(|err| panic!("invalid permission bit in `PermissionIter`\n{err}"), Some)
	}

	fn size_hint(&self) -> (usize, Option<usize>)
	{
		let count = self.bits.count_ones() as usize;
		(count, Some(count))
	}

	fn count(self) -> usize
	where
		Self: Sized,
	{
		self.bits.count_ones() as usize
	}
}

impl ExactSizeIterator for Iter
{
}
