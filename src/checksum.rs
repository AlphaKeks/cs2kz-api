//! Checksums
//!
//! This module contains the [`Checksum`] struct which is a shared abstraction
//! for opaque checksums. It implements all the necessary traits that downstream
//! consumers may need.
//!
//! The current implementation uses [MD5], but that may change in the future.
//!
//! [MD5]: ::md5

use {
	md5::{Digest, Md5},
	serde::{Deserialize, Deserializer, Serialize, Serializer},
	std::{array, fmt, io, num::ParseIntError, str::FromStr},
};

/// The number of bytes that make up a [`Checksum`]
const RAW_LEN: usize = 16_usize;

/// The number of bytes used by a [`Checksum`] when represented as a string
const STR_LEN: usize = RAW_LEN * 2;

/// An opaque checksum
///
/// See [module-level documentation] for more information.
///
/// [module-level documentation]: crate::checksum
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum
{
	bytes: [u8; RAW_LEN],
}

/// Error for parsing strings into [`Checksum`]s
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
	pub const fn as_bytes(&self) -> &[u8]
	{
		self.bytes.as_slice()
	}

	/// Computes a [`Checksum`] from the given `bytes`.
	pub fn from_bytes(bytes: &[u8]) -> Self
	{
		let mut hasher = Md5::default();
		hasher.update(bytes);

		Self { bytes: hasher.finalize().into() }
	}

	/// Computes a [`Checksum`] from all bytes read from the given `reader`
	/// (until EOF).
	#[instrument(level = "debug", skip(reader), err)]
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

impl FromStr for Checksum
{
	type Err = ParseChecksumError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.len() != STR_LEN {
			return Err(ParseChecksumError::InvalidLength { got: value.len() });
		}

		let bytes = array::try_from_fn(|idx| {
			let substr = value.get(idx * 2..(idx + 1) * 2).unwrap_or_else(|| {
				unreachable!("we checked the input's length");
			});

			u8::from_str_radix(substr, 16).map_err(ParseChecksumError::ParseHexDigit)
		})?;

		Ok(Self { bytes })
	}
}

impl_rand!(Checksum => |rng| Checksum { bytes: rng.random() });

impl Serialize for Checksum
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		if serializer.is_human_readable() {
			format_args!("{self}").serialize(serializer)
		} else {
			self.as_bytes().serialize(serializer)
		}
	}
}

impl<'de> Deserialize<'de> for Checksum
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		use serde::de;

		struct ChecksumVisitor;

		impl de::Visitor<'_> for ChecksumVisitor
		{
			type Value = Checksum;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
			{
				fmt.write_str("a checksum")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				value.parse().map_err(E::custom)
			}

			fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				<[u8; RAW_LEN]>::try_from(value)
					.map(|bytes| Checksum { bytes })
					.map_err(|_| E::invalid_length(value.len(), &self))
			}
		}

		if deserializer.is_human_readable() {
			deserializer.deserialize_str(ChecksumVisitor)
		} else {
			deserializer.deserialize_bytes(ChecksumVisitor)
		}
	}
}

impl_sqlx!(Checksum => {
	Type as [u8];
	Encode<'q, 'a> as &'a [u8] = |checksum| checksum.as_bytes();
	Decode<'r> as &'r [u8] = |bytes| {
		<&[u8] as TryInto<[u8; RAW_LEN]>>::try_into(bytes)
			.map(|bytes| Checksum { bytes })
	};
});

impl_utoipa!(Checksum => {
	Object::builder()
		.description(Some("an MD5 checksum"))
		.schema_type(schema::Type::String)
		.min_length(Some(STR_LEN))
		.max_length(Some(STR_LEN))
		.examples(["ba29b1da0f9c28e2a9e072aba46cf040"])
});
