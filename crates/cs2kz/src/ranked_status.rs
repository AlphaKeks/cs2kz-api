use std::fmt;
use std::str::FromStr;

/// The different states a course filter can be in.
#[repr(i8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RankedStatus {
	/// The filter will never be [ranked], as per the mapper's request.
	///
	/// [ranked]: RankedStatus::Ranked
	Never = -1,

	/// The filter is currently not ranked, but not because it was explicitly
	/// requested, just because it didn't meet requirements.
	Unranked = 0,

	/// The filter is ranked.
	Ranked = 1,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown ranked status")]
pub struct UnknownRankedStatus;

impl RankedStatus {
	pub const fn is_ranked(&self) -> bool {
		matches!(self, Self::Ranked)
	}
}

impl fmt::Display for RankedStatus {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::Never => "never",
			Self::Unranked => "unranked",
			Self::Ranked => "ranked",
		})
	}
}

impl From<RankedStatus> for i8 {
	fn from(ranked_status: RankedStatus) -> Self {
		ranked_status as i8
	}
}

impl TryFrom<i8> for RankedStatus {
	type Error = UnknownRankedStatus;

	fn try_from(int: i8) -> Result<Self, Self::Error> {
		match int {
			-1 => Ok(Self::Never),
			0 => Ok(Self::Unranked),
			1 => Ok(Self::Ranked),
			_ => Err(UnknownRankedStatus),
		}
	}
}

impl FromStr for RankedStatus {
	type Err = UnknownRankedStatus;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"-1" | "never" => Ok(Self::Never),
			"0" | "unranked" => Ok(Self::Unranked),
			"1" | "ranked" => Ok(Self::Ranked),
			_ => Err(UnknownRankedStatus),
		}
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<RankedStatus> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> RankedStatus
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(-1..=1)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for RankedStatus {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_str(serializer)
	}
}

#[cfg(feature = "serde")]
impl RankedStatus {
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
				Self::Never => "never",
				Self::Unranked => "unranked",
				Self::Ranked => "ranked",
			},
			serializer,
		)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for RankedStatus {
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
					&"a cs2kz ranked status",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl RankedStatus {
	pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<i8 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Signed(i64::from(value)),
					&"a cs2kz ranked status",
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
				"never" => Ok(Self::Never),
				"unranked" => Ok(Self::Unranked),
				"ranked" => Ok(Self::Ranked),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz ranked status",
				)),
			}
		})
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for RankedStatus
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
impl<'q, DB> sqlx::Encode<'q, DB> for RankedStatus
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
impl<'r, DB> sqlx::Decode<'r, DB> for RankedStatus
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
