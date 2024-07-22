//! Custom wrapper around [`semver::Version`] so we can override trait
//! implementations.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A CS2KZ plugin version.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PluginVersion(pub semver::Version);

impl fmt::Display for PluginVersion
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt::Display::fmt(&self.0, f)
	}
}

impl Serialize for PluginVersion
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for PluginVersion
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let mut s = &*String::deserialize(deserializer)?;

		if s.starts_with('v') {
			s = &s[1..];
		}

		s.parse::<semver::Version>()
			.map(Self)
			.map_err(serde::de::Error::custom)
	}
}

impl<DB> sqlx::Type<DB> for PluginVersion
where
	DB: sqlx::Database,
	String: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<String as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<String as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for PluginVersion
where
	DB: sqlx::Database,
	String: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		<String as sqlx::Encode<'q, DB>>::encode_by_ref(&self.0.to_string(), buf)
	}

	fn encode(
		self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	where
		Self: Sized,
	{
		<String as sqlx::Encode<'q, DB>>::encode(self.0.to_string(), buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		<String as sqlx::Encode<'q, DB>>::produces(&self.0.to_string())
	}

	fn size_hint(&self) -> usize
	{
		<String as sqlx::Encode<'q, DB>>::size_hint(&self.0.to_string())
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PluginVersion
where
	DB: sqlx::Database,
	String: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		<String as sqlx::Decode<'r, DB>>::decode(value)?
			.parse::<semver::Version>()
			.map(Self)
			.map_err(Into::into)
	}
}
