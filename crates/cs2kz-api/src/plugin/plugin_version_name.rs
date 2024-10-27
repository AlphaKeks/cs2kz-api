/// A cs2kz-metamod version name.
#[derive(Debug, Clone)]
pub enum PluginVersionName {
	/// Local development build.
	#[cfg(not(feature = "production"))]
	Dev,

	/// A SemVer version identifier (e.g. `0.14.3`).
	SemVer(semver::Version),
}

impl PluginVersionName {
	/// # Panics
	///
	/// This function will panic if
	///    1. it is called in a production environment
	///    2. `self` is the special `Dev` version
	pub fn into_semver(self) -> semver::Version {
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => panic!("cannot turn `dev` into a semver identifier"),
			Self::SemVer(version) => version,
		}
	}

	/// # Panics
	///
	/// This function will panic if
	///    1. it is called in a production environment
	///    2. `self` is the special `Dev` version
	pub fn as_semver(&self) -> &semver::Version {
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => panic!("cannot turn `dev` into a semver identifier"),
			Self::SemVer(version) => version,
		}
	}
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
		let str = <&'de str as serde::Deserialize<'de>>::deserialize(deserializer)?;

		#[cfg(not(feature = "production"))]
		{
			if str == "dev" {
				return Ok(Self::Dev);
			}
		}

		str.parse::<semver::Version>()
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
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	#[instrument(level = "trace", skip(buf), err(level = "debug"))]
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => <&str as sqlx::Encode<'q, DB>>::encode_by_ref(&"dev", buf),
			Self::SemVer(version) => {
				<&str as sqlx::Encode<'q, DB>>::encode_by_ref(&&*version.to_string(), buf)
			},
		}
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => <&str as sqlx::Encode<'q, DB>>::produces(&"dev"),
			Self::SemVer(version) => {
				<&str as sqlx::Encode<'q, DB>>::produces(&&*version.to_string())
			},
		}
	}

	fn size_hint(&self) -> usize {
		match self {
			#[cfg(not(feature = "production"))]
			Self::Dev => <&str as sqlx::Encode<'q, DB>>::size_hint(&"dev"),
			Self::SemVer(version) => {
				<&str as sqlx::Encode<'q, DB>>::size_hint(&&*version.to_string())
			},
		}
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for PluginVersionName
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	#[instrument(level = "trace", skip_all, ret(level = "debug"), err(level = "debug"))]
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		let str = <&'r str as sqlx::Decode<'r, DB>>::decode(value)?;

		if cfg!(feature = "production") {
			assert_ne!(str, "dev", "production database should not have a `dev` plugin version");
		}

		#[cfg(not(feature = "production"))]
		{
			if str == "dev" {
				return Ok(Self::Dev);
			}
		}

		str.parse::<semver::Version>()
			.map(Self::SemVer)
			.map_err(Into::into)
	}
}

mod utoipa_impls {
	use utoipa::openapi::{ObjectBuilder, RefOr, Schema, schema};
	use utoipa::{PartialSchema, ToSchema};

	use super::PluginVersionName;

	impl PartialSchema for PluginVersionName {
		fn schema() -> RefOr<Schema> {
			Schema::Object(
				ObjectBuilder::new()
					.schema_type(schema::Type::String)
					.title(Some("Plugin Version Name"))
					.description(Some("a SemVer identifier"))
					.examples(["0.3.1"])
					.build(),
			)
			.into()
		}
	}

	impl ToSchema for PluginVersionName {}
}
