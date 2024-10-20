use std::fmt;
use std::str::FromStr;

/// A CS2KZ mode.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
	/// The VNL mode.
	Vanilla = 1,

	/// The CKZ mode.
	Classic = 2,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown mode")]
pub struct UnknownMode;

impl Mode {
	pub const fn is_vanilla(&self) -> bool {
		matches!(self, Mode::Vanilla)
	}

	pub const fn is_classic(&self) -> bool {
		matches!(self, Mode::Classic)
	}
}

impl fmt::Display for Mode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match self {
			Mode::Vanilla => "VNL",
			Mode::Classic => "CKZ",
		})
	}
}

impl From<Mode> for u8 {
	fn from(mode: Mode) -> Self {
		mode as u8
	}
}

impl TryFrom<u8> for Mode {
	type Error = UnknownMode;

	fn try_from(value: u8) -> Result<Self, Self::Error> {
		match value {
			1 => Ok(Mode::Vanilla),
			2 => Ok(Mode::Classic),
			_ => Err(UnknownMode),
		}
	}
}

impl FromStr for Mode {
	type Err = UnknownMode;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		match value {
			"1" | "vnl" | "VNL" | "vanilla" | "Vanilla" => Ok(Mode::Vanilla),
			"2" | "ckz" | "CKZ" | "classic" | "Classic" => Ok(Mode::Classic),
			_ => Err(UnknownMode),
		}
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<Mode> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> Mode
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(1..=2)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for Mode {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_long(serializer)
	}
}

#[cfg(feature = "serde")]
impl Mode {
	pub fn serialize_int<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<u8 as serde::Serialize>::serialize(&u8::from(*self), serializer)
	}

	pub fn serialize_short<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<str as serde::Serialize>::serialize(
			match self {
				Mode::Vanilla => "vnl",
				Mode::Classic => "ckz",
			},
			serializer,
		)
	}

	pub fn serialize_long<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		<str as serde::Serialize>::serialize(
			match self {
				Mode::Vanilla => "vanilla",
				Mode::Classic => "classic",
			},
			serializer,
		)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Mode {
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
					&"a cs2kz mode ID",
				)
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl Mode {
	pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u8 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(u64::from(value)),
					&"a cs2kz mode ID",
				)
			})
		})
	}

	pub fn deserialize_short<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			match value {
				"vnl" | "VNL" => Ok(Self::Vanilla),
				"ckz" | "CKZ" => Ok(Self::Classic),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz mode",
				)),
			}
		})
	}

	pub fn deserialize_long<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<&'de str as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			match value {
				"vanilla" | "Vanilla" => Ok(Self::Vanilla),
				"classic" | "Classic" => Ok(Self::Classic),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz mode",
				)),
			}
		})
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for Mode
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
impl<'q, DB> sqlx::Encode<'q, DB> for Mode
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
impl<'r, DB> sqlx::Decode<'r, DB> for Mode
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
