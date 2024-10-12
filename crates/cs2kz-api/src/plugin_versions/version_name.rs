use std::io::{self, Write as _};

#[derive(Debug, Display, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginVersionName {
	/// A temporary version name we use during local testing. This variant is not enabled when
	/// compiling for production.
	#[cfg(not(feature = "production"))]
	#[display("dev")]
	Dev,

	/// Any valid SemVer version.
	SemVer(semver::Version),
}

impl serde::Serialize for PluginVersionName {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => "dev".serialize(serializer),
			Self::SemVer(version) => version.serialize(serializer),
		}
	}
}

impl<'de> serde::Deserialize<'de> for PluginVersionName {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let value = <&str>::deserialize(deserializer)?;

		#[cfg(not(feature = "production"))]
		if value == "dev" {
			return Ok(Self::Dev);
		}

		value
			.parse::<semver::Version>()
			.map(Self::SemVer)
			.map_err(serde::de::Error::custom)
	}
}

impl<DB> sqlx::Type<DB> for PluginVersionName
where
	DB: sqlx::Database,
	str: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<str as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<str as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for PluginVersionName
where
	DB: sqlx::Database,
	<DB as sqlx::Database>::ArgumentBuffer<'q>: io::Write,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		Ok(match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => <&str as sqlx::Encode<'q, DB>>::encode_by_ref(&"dev", buf)?,
			Self::SemVer(version) => {
				write!(buf, "{version}")?;
				sqlx::encode::IsNull::No
			}
		})
	}

	fn size_hint(&self) -> usize {
		match self {
			// the 3 bytes that make up "dev"
			#[cfg(not(feature = "production"))]
			Self::Dev => 3,

			// estimate for aaa.bbb.cccc
			Self::SemVer(_) => 12,
		}
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PluginVersionName
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		let value = <&'r str as sqlx::Decode<'r, DB>>::decode(value)?;

		#[cfg(not(feature = "production"))]
		if value == "dev" {
			return Ok(Self::Dev);
		}

		value
			.parse::<semver::Version>()
			.map(Self::SemVer)
			.map_err(Into::into)
	}
}
