//! Checksums
//!
//! This module contains the [`Checksum`] struct which is a shared abstraction
//! for opaque checksums. It implements all the necessary traits that downstream
//! consumers may need.
//!
//! The current implementation uses [MD5], but that may change in the future.
//!
//! [MD5]: ::md5::Md5

use std::{array, fmt, io, num::ParseIntError, ops::Deref, str::FromStr};

use md5::{Digest, Md5};

const RAW_LEN: usize = 16_usize;
const STR_LEN: usize = RAW_LEN * 2;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum
{
	bytes: [u8; RAW_LEN],
}

#[derive(Debug, Display, Error)]
pub enum ParseChecksumError
{
	#[display("invalid length; expected {STR_LEN} but got {got}")]
	InvalidLength
	{
		#[error(ignore)]
		got: usize,
	},

	#[display("failed to parse hex digit: {_0}")]
	ParseHexDigit(ParseIntError),
}

impl Checksum
{
	/// Computes a [`Checksum`] from the given `bytes`.
	pub fn from_bytes(bytes: &[u8]) -> Self
	{
		let mut hasher = Md5::default();
		hasher.update(bytes);

		Self { bytes: hasher.finalize().into() }
	}

	/// Computes a [`Checksum`] from all bytes read until EOF from the given
	/// `reader`.
	#[tracing::instrument(level = "trace", skip(reader), err(level = "warn"))]
	pub fn from_reader<R>(reader: &mut R) -> io::Result<Self>
	where
		R: ?Sized + io::Read,
	{
		let mut hasher = Md5::default();
		io::copy(reader, &mut hasher)?;

		Ok(Self { bytes: hasher.finalize().into() })
	}
}

impl fmt::Debug for Checksum
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_tuple("Checksum")
			.field_with(|fmt| fmt::Display::fmt(self, fmt))
			.finish()
	}
}

impl fmt::Display for Checksum
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		self.bytes.iter().try_for_each(|byte| write!(fmt, "{byte:02x}"))
	}
}

impl Deref for Checksum
{
	type Target = [u8];

	fn deref(&self) -> &Self::Target
	{
		&self.bytes[..]
	}
}

impl FromStr for Checksum
{
	type Err = ParseChecksumError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.len() != STR_LEN {
			return Err(ParseChecksumError::InvalidLength { got: value.len() });
		}

		Ok(Self {
			bytes: array::try_from_fn(|idx| {
				let substr = value
					.get(idx * 2..(idx + 1) * 2)
					.unwrap_or_else(|| panic!("we checked the input's length"));

				u8::from_str_radix(substr, 16).map_err(ParseChecksumError::ParseHexDigit)
			})?,
		})
	}
}

impl serde::Serialize for Checksum
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for Checksum
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		if !deserializer.is_human_readable() {
			return <[u8; RAW_LEN]>::deserialize(deserializer).map(|bytes| Self { bytes });
		}

		<String as serde::Deserialize<'de>>::deserialize(deserializer)?
			.parse::<Self>()
			.map_err(|err| match err {
				ParseChecksumError::InvalidLength { got } => {
					serde::de::Error::invalid_length(got, &"32 hex characters")
				},
				ParseChecksumError::ParseHexDigit(error) => serde::de::Error::custom(error),
			})
	}
}

impl_rand!(Checksum => |rng| Checksum { bytes: rng.random() });

impl<DB> sqlx::Type<DB> for Checksum
where
	DB: sqlx::Database,
	[u8]: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<[u8] as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<[u8] as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Checksum
where
	DB: sqlx::Database,
	for<'a> &'a [u8]: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn std::error::Error + Send + Sync>>
	{
		(&&**self).encode(buf)
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		(&&**self).produces()
	}

	fn size_hint(&self) -> usize
	{
		(&&**self).size_hint()
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Checksum
where
	DB: sqlx::Database,
	&'r [u8]: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn std::error::Error + Send + Sync>>
	{
		let bytes = <&'r [u8] as sqlx::Decode<'r, DB>>::decode(value)?;
		let bytes = <[u8; RAW_LEN]>::try_from(bytes)?;

		Ok(Self { bytes })
	}
}

impl utoipa::ToSchema for Checksum
{
}

impl utoipa::PartialSchema for Checksum
{
	fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>
	{
		use utoipa::openapi::{
			Object,
			schema::{self, SchemaType},
		};

		Object::builder()
			.description(Some("an MD5 checksum"))
			.schema_type(SchemaType::Type(schema::Type::String))
			.min_length(Some(STR_LEN))
			.max_length(Some(STR_LEN))
			.examples(["ba29b1da0f9c28e2a9e072aba46cf040"])
			.into()
	}
}
