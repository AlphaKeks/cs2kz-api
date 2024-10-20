use std::fmt;
use std::str::FromStr;

/// The 10 difficulty ratings for CS2KZ course filters.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
	/// The lowest tier.
	///
	/// Someone who has never played KZ before should be able to complete this.
	VeryEasy = 1,

	/// Requires some prior KZ knowledge, such as air strafing and bunnyhopping.
	Easy = 2,

	/// Players who have the KZ basics down should be able to complete this.
	Medium = 3,

	/// Players who have played KZ consistently for a while and are starting to
	/// learn more advanced techniques like ladders and surfs.
	Advanced = 4,

	/// Just like [Advanced], but harder.
	///
	/// [Advanced]: type@Tier::Advanced
	Hard = 5,

	/// Just like [Hard], but very.
	///
	/// [Hard]: type@Tier::Hard
	VeryHard = 6,

	/// For players with a lot of KZ experience who want to challenge
	/// themselves. Getting a top time on these requires mastering KZ.
	Extreme = 7,

	/// These are the hardest in the game, and only very good KZ players can
	/// complete these at all.
	Death = 8,

	/// Technically possible, but not feasible for humans. This tier is reserved
	/// for TAS runs, and any runs submitted by humans will be reviewed for
	/// cheats.
	Unfeasible = 9,

	/// Technically impossible. Even with perfect inputs.
	Impossible = 10,
}

#[non_exhaustive]
#[derive(Debug, Error)]
#[error("unknown tier")]
pub struct UnknownTier;

impl Tier {
	/// Checks if this tier is in the humanly-possible range.
	pub const fn is_humanly_possible(&self) -> bool {
		(*self as u8) < (Tier::Unfeasible as u8)
	}
}

impl fmt::Display for Tier {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(match (self, f.alternate()) {
			(Tier::VeryEasy, false) => "1",
			(Tier::VeryEasy, true) => "Very Easy",
			(Tier::Easy, false) => "2",
			(Tier::Easy, true) => "Easy",
			(Tier::Medium, false) => "3",
			(Tier::Medium, true) => "Medium",
			(Tier::Advanced, false) => "4",
			(Tier::Advanced, true) => "Advanced",
			(Tier::Hard, false) => "5",
			(Tier::Hard, true) => "Hard",
			(Tier::VeryHard, false) => "6",
			(Tier::VeryHard, true) => "Very Hard",
			(Tier::Extreme, false) => "7",
			(Tier::Extreme, true) => "Extreme",
			(Tier::Death, false) => "8",
			(Tier::Death, true) => "Death",
			(Tier::Unfeasible, false) => "9",
			(Tier::Unfeasible, true) => "Unfeasible",
			(Tier::Impossible, false) => "10",
			(Tier::Impossible, true) => "Impossible",
		})
	}
}

impl From<Tier> for u8 {
	fn from(tier: Tier) -> Self {
		tier as u8
	}
}

impl TryFrom<u8> for Tier {
	type Error = UnknownTier;

	fn try_from(int: u8) -> Result<Self, Self::Error> {
		match int {
			1 => Ok(Tier::VeryEasy),
			2 => Ok(Tier::Easy),
			3 => Ok(Tier::Medium),
			4 => Ok(Tier::Advanced),
			5 => Ok(Tier::Hard),
			6 => Ok(Tier::VeryHard),
			7 => Ok(Tier::Extreme),
			8 => Ok(Tier::Death),
			9 => Ok(Tier::Unfeasible),
			10 => Ok(Tier::Impossible),
			_ => Err(UnknownTier),
		}
	}
}

impl FromStr for Tier {
	type Err = UnknownTier;

	fn from_str(value: &str) -> Result<Self, Self::Err> {
		if let Ok(int) = value.parse::<u8>() {
			return Self::try_from(int);
		}

		match value {
			"very_easy" => Ok(Tier::VeryEasy),
			"easy" => Ok(Tier::Easy),
			"medium" => Ok(Tier::Medium),
			"advanced" => Ok(Tier::Advanced),
			"hard" => Ok(Tier::Hard),
			"very_hard" => Ok(Tier::VeryHard),
			"extreme" => Ok(Tier::Extreme),
			"death" => Ok(Tier::Death),
			"unfeasible" => Ok(Tier::Unfeasible),
			"impossible" => Ok(Tier::Impossible),
			_ => Err(UnknownTier),
		}
	}
}

#[cfg(feature = "rand")]
impl rand::distributions::Distribution<Tier> for rand::distributions::Standard {
	fn sample<R>(&self, rng: &mut R) -> Tier
	where
		R: rand::Rng + ?Sized,
	{
		rng.gen_range(1..=10)
			.try_into()
			.expect("RNG generated out-of-range value")
	}
}

#[cfg(feature = "serde")]
impl serde::Serialize for Tier {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.serialize_str(serializer)
	}
}

#[cfg(feature = "serde")]
impl Tier {
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
				Tier::VeryEasy => "very_easy",
				Tier::Easy => "easy",
				Tier::Medium => "medium",
				Tier::Advanced => "advanced",
				Tier::Hard => "hard",
				Tier::VeryHard => "very_hard",
				Tier::Extreme => "extreme",
				Tier::Death => "death",
				Tier::Unfeasible => "unfeasible",
				Tier::Impossible => "impossible",
			},
			serializer,
		)
	}
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Tier {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		use crate::either::Either;

		Either::<u8, String>::deserialize(deserializer).and_then(|value| match value {
			Either::A(int) => Self::try_from(int).map_err(|_| {
				de::Error::invalid_value(de::Unexpected::Unsigned(u64::from(int)), &"a cs2kz tier")
			}),
			Either::B(string) => string.parse::<Self>().map_err(de::Error::custom),
		})
	}
}

#[cfg(feature = "serde")]
impl Tier {
	pub fn deserialize_int<'de, D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<u8 as serde::Deserialize<'de>>::deserialize(deserializer).and_then(|value| {
			value.try_into().map_err(|_| {
				serde::de::Error::invalid_value(
					serde::de::Unexpected::Unsigned(u64::from(value)),
					&"a cs2kz tier",
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
				"very_easy" => Ok(Tier::VeryEasy),
				"easy" => Ok(Tier::Easy),
				"medium" => Ok(Tier::Medium),
				"advanced" => Ok(Tier::Advanced),
				"hard" => Ok(Tier::Hard),
				"very_hard" => Ok(Tier::VeryHard),
				"extreme" => Ok(Tier::Extreme),
				"death" => Ok(Tier::Death),
				"unfeasible" => Ok(Tier::Unfeasible),
				"impossible" => Ok(Tier::Impossible),
				_ => Err(serde::de::Error::invalid_value(
					serde::de::Unexpected::Str(value),
					&"a cs2kz tier",
				)),
			}
		})
	}
}

#[cfg(feature = "sqlx")]
impl<DB> sqlx::Type<DB> for Tier
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
impl<'q, DB> sqlx::Encode<'q, DB> for Tier
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
impl<'r, DB> sqlx::Decode<'r, DB> for Tier
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
