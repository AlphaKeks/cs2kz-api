use std::fmt;
use std::str::{self, FromStr};

use hex::FromHexError;

#[derive(Clone, Copy, utoipa::ToSchema)]
#[schema(value_type = str)]
pub struct Revision([u8; 20]);

impl fmt::Debug for Revision {
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut buf = [0; 40];
		hex::encode_to_slice(&self.0[..], &mut buf)
			.expect("buffer is twice as large as source slice");

		str::from_utf8(&buf[..])
			.map(|revision| fmt.write_str(revision))
			.expect("hex-slice should be valid UTF-8")
	}
}

#[derive(Debug, Error)]
pub enum ParseGitRevision {
	#[error("invalid length for git revision")]
	InvalidLength,

	#[error("git revision must be ascii")]
	NotAscii,

	#[error(transparent)]
	ParseAsHex(#[from] FromHexError),
}

impl FromStr for Revision {
	type Err = ParseGitRevision;

	fn from_str(str: &str) -> Result<Self, Self::Err> {
		if str.len() != 40 {
			return Err(ParseGitRevision::InvalidLength);
		}

		if !str.is_ascii() {
			return Err(ParseGitRevision::NotAscii);
		}

		let mut buf = [0; 20];

		hex::decode_to_slice(str, &mut buf[..]).map_err(ParseGitRevision::ParseAsHex)?;

		Ok(Self(buf))
	}
}

impl serde::Serialize for Revision {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format_args!("{self:?}").serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Revision {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		let str = <&str as serde::Deserialize<'de>>::deserialize(deserializer)?;

		str.parse::<Self>().map_err(|error| match error {
			ParseGitRevision::InvalidLength => de::Error::invalid_length(str.len(), &"40"),
			ParseGitRevision::NotAscii | ParseGitRevision::ParseAsHex(_) => {
				de::Error::custom(error)
			},
		})
	}
}

impl<DB> sqlx::Type<DB> for Revision
where
	DB: sqlx::Database,
	[u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo {
		<[u8] as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool {
		<[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Revision
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
{
	#[instrument(level = "trace", skip(buf), err(level = "debug"))]
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Sync + Send>> {
		<&[u8] as sqlx::Encode<'q, DB>>::encode_by_ref(&&self.0[..], buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo> {
		<&[u8] as sqlx::Encode<'q, DB>>::produces(&&self.0[..])
	}

	fn size_hint(&self) -> usize {
		<&[u8] as sqlx::Encode<'q, DB>>::size_hint(&&self.0[..])
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Revision
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	#[instrument(level = "trace", skip_all, ret(level = "debug"), err(level = "debug"))]
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
		<&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)?
			.try_into()
			.map(Self)
			.map_err(Into::into)
	}
}
