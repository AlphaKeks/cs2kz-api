use std::fmt;
use std::str::FromStr;

/// The different types of jumps in CS2KZ.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum JumpType {
	/// LJ
	LongJump = 1,

	/// BH
	Bhop = 2,

	/// MBH
	MultiBhop = 3,

	/// WJ
	WeirdJump = 4,

	/// LAJ
	LadderJump = 5,

	/// LAH
	Ladderhop = 6,

	/// JB
	Jumpbug = 7,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown jump type")]
pub struct UnknownJumpType;

impl fmt::Display for JumpType {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Self::LongJump => "LJ",
			Self::Bhop => "BH",
			Self::MultiBhop => "MBH",
			Self::WeirdJump => "WJ",
			Self::LadderJump => "LAJ",
			Self::Ladderhop => "LAH",
			Self::Jumpbug => "JB",
		})
	}
}

impl From<JumpType> for u8 {
	fn from(jump_type: JumpType) -> Self {
		jump_type as u8
	}
}

impl TryFrom<u8> for JumpType {
	type Error = UnknownJumpType;

	fn try_from(int: u8) -> Result<Self, Self::Error> {
		match int {
			1 => Ok(JumpType::LongJump),
			2 => Ok(JumpType::Bhop),
			3 => Ok(JumpType::MultiBhop),
			4 => Ok(JumpType::WeirdJump),
			5 => Ok(JumpType::LadderJump),
			6 => Ok(JumpType::Ladderhop),
			7 => Ok(JumpType::Jumpbug),
			_ => Err(UnknownJumpType),
		}
	}
}

impl FromStr for JumpType {
	type Err = UnknownJumpType;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(int) = value.parse::<u8>() {
			return Self::try_from(int);
		}

		match value {
			"lj" | "LJ" | "long_jump" => Ok(JumpType::LongJump),
			"bh" | "BH" | "bhop" => Ok(JumpType::Bhop),
			"mbh" | "MBH" | "multi_bhop" => Ok(JumpType::MultiBhop),
			"wj" | "WJ" | "weird_jump" => Ok(JumpType::WeirdJump),
			"laj" | "LAJ" | "ladder_jump" => Ok(JumpType::LadderJump),
			"lah" | "LAH" | "ladder_hop" => Ok(JumpType::Ladderhop),
			"jb" | "JB" | "jump_bug" => Ok(JumpType::Jumpbug),
			_ => Err(UnknownJumpType),
		}
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<JumpType> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> JumpType
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(1..=7)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for JumpType {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_str(serializer)
	}
}

#[cfg(feature = "serde")]
impl JumpType {
	pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<u8 as serde::Serialize>::serialize(&u8::from(*self), serializer)
	}

	pub fn serialize_str<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<str as serde::Serialize>::serialize(
			match self {
				JumpType::LongJump => "long_jump",
				JumpType::Bhop => "bhop",
				JumpType::MultiBhop => "multi_bhop",
				JumpType::WeirdJump => "weird_jump",
				JumpType::LadderJump => "ladder_jump",
				JumpType::Ladderhop => "ladder_hop",
				JumpType::Jumpbug => "jump_bug",
			},
			serializer,
		)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for JumpType {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::either::Either;

		Either::<u8, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(int) => Self::try_from(int).map_err(|_| {
				de::Error::invalid_value(
					de::Unexpected::Unsigned(u64::from(int)),
					&"a cs2kz jump type",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl JumpType {
	pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u8 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(u64::from(value)),
					&"a cs2kz jump type",
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
				"long_jump" => Ok(JumpType::LongJump),
				"bhop" => Ok(JumpType::Bhop),
				"multi_bhop" => Ok(JumpType::MultiBhop),
				"weird_jump" => Ok(JumpType::WeirdJump),
				"ladder_jump" => Ok(JumpType::LadderJump),
				"ladder_hop" => Ok(JumpType::Ladderhop),
				"jump_bug" => Ok(JumpType::Jumpbug),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz jump type",
				)),
			}
		})
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for JumpType
where
	DB: sqlx::Database,
	u8: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<u8 as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<u8 as sqlx::Type<DB>>::compatible(ty)
	}
}

#[cfg(feature = "sqlx")]
impl<'q, DB> sqlx::Encode<'q, DB> for JumpType
where
	DB: sqlx::Database,
	u8: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u8 as sqlx::Encode<'q, DB>>::encode_by_ref(&u8::from(*self), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<u8 as sqlx::Encode<'q, DB>>::encode(u8::from(self), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<u8 as sqlx::Encode<'q, DB>>::produces(&u8::from(*self))
	}

	fn size_hint(&self) -> usize {
		<u8 as sqlx::Encode<'q, DB>>::size_hint(&u8::from(*self))
	}
}

#[cfg(feature = "sqlx")]
impl<'r, DB> sqlx::Decode<'r, DB> for JumpType
where
	DB: sqlx::Database,
	u8: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<u8 as sqlx::Decode<'r, DB>>::decode(value)
			.and_then(|value| value.try_into().map_err(Into::into))
	}
}
