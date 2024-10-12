use std::fmt;
use std::str::FromStr;

/// The different states a global map can be in.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MapApprovalStatus {
	/// The map is disabled.
	///
	/// This means it will be excluded by the API for most actions.
	/// This status is applied after the fact, e.g. when a map-breaking bug is found.
	Disabled = -1,

	/// The map is in public testing.
	///
	/// This means people can submit records on the map, but leaderboards may be reset at any
	/// point, and players cannot gain points or WRs on the map. The map may also still change as
	/// player feedback is coming in.
	InTesting = 0,

	/// The map is approved.
	///
	/// This is the final stage of the approval process.
	Approved = 1,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown map approval status")]
pub struct UnknownMapApprovalStatus;

impl MapApprovalStatus {
	pub const fn is_disabled(&self) -> bool {
		matches!(self, MapApprovalStatus::Disabled)
	}

	pub const fn is_in_testing(&self) -> bool {
		matches!(self, MapApprovalStatus::InTesting)
	}

	pub const fn is_approved(&self) -> bool {
		matches!(self, MapApprovalStatus::Approved)
	}
}

impl fmt::Display for MapApprovalStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Disabled => "disabled",
			Self::InTesting => "in testing",
			Self::Approved => "approved",
		})
	}
}

impl From<MapApprovalStatus> for i8 {
	fn from(approval_status: MapApprovalStatus) -> Self {
		approval_status as i8
	}
}

impl TryFrom<i8> for MapApprovalStatus {
	type Error = UnknownMapApprovalStatus;

	fn try_from(int: i8) -> Result<Self, Self::Error> {
		match int {
			-1 => Ok(Self::Disabled),
			0 => Ok(Self::InTesting),
			1 => Ok(Self::Approved),
			_ => Err(UnknownMapApprovalStatus),
		}
	}
}

impl FromStr for MapApprovalStatus {
	type Err = UnknownMapApprovalStatus;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"-1" | "disabled" => Ok(Self::Disabled),
			"0" | "in_testing" => Ok(Self::InTesting),
			"1" | "approved" => Ok(Self::Approved),
			_ => Err(UnknownMapApprovalStatus),
		}
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<MapApprovalStatus> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> MapApprovalStatus
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(-1..=1)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for MapApprovalStatus {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_str(serializer)
	}
}

#[cfg(feature = "serde")]
impl MapApprovalStatus {
	pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<i8 as serde::Serialize>::serialize(&i8::from(*self), serializer)
	}

	pub fn serialize_str<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<str as serde::Serialize>::serialize(
			match self {
				Self::Disabled => "disabled",
				Self::InTesting => "in_testing",
				Self::Approved => "approved",
			},
			serializer,
		)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for MapApprovalStatus {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::either::Either;

		Either::<i8, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(int) => Self::try_from(int).map_err(|_| {
				de::Error::invalid_value(
					de::Unexpected::Signed(i64::from(int)),
					&"a cs2kz map approval status",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl MapApprovalStatus {
	pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<i8 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Signed(i64::from(value)),
					&"a cs2kz map approval status",
				)
			})
		})
	}

	pub fn deserialize_str<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			match value {
				"disabled" => Ok(Self::Disabled),
				"in_testing" => Ok(Self::InTesting),
				"approved" => Ok(Self::Approved),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz map approval status",
				)),
			}
		})
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for MapApprovalStatus
where
	DB: sqlx::Database,
	i8: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<i8 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<i8 as sqlx::Type<DB>>::compatible(ty)
	}
}

#[cfg(feature = "sqlx")]
impl<'q, DB> sqlx::Encode<'q, DB> for MapApprovalStatus
where
	DB: sqlx::Database,
	i8: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<i8 as sqlx::Encode<'q, DB>>::encode_by_ref(&i8::from(*self), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<i8 as sqlx::Encode<'q, DB>>::encode(i8::from(self), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<i8 as sqlx::Encode<'q, DB>>::produces(&i8::from(*self))
	}

	fn size_hint(&self) -> usize {
		<i8 as sqlx::Encode<'q, DB>>::size_hint(&i8::from(*self))
	}
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for MapApprovalStatus
where
	DB: sqlx::Database,
	i8: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<i8 as sqlx::Decode<'r, DB>>::decode(value)
			.and_then(|value| value.try_into().map_err(Into::into))
	}
}
