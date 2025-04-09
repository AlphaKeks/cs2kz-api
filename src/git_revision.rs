use std::{array, fmt, num::ParseIntError, str::FromStr};

const RAW_LEN: usize = 20_usize;
const STR_LEN: usize = RAW_LEN * 2;

/// A git revision stored as raw bytes
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GitRevision
{
	bytes: [u8; RAW_LEN],
}

/// Error for parsing strings into [`GitRevision`]s
#[derive(Debug, Display, Error)]
pub enum ParseGitRevisionError
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

impl GitRevision
{
	pub const fn as_bytes(&self) -> &[u8]
	{
		self.bytes.as_slice()
	}
}

impl fmt::Debug for GitRevision
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		fmt.debug_tuple("GitRevision")
			.field_with(|fmt| fmt::Display::fmt(self, fmt))
			.finish()
	}
}

impl fmt::Display for GitRevision
{
	fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		self.bytes.iter().try_for_each(|byte| write!(fmt, "{byte:02x}"))
	}
}

impl FromStr for GitRevision
{
	type Err = ParseGitRevisionError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if value.len() != STR_LEN {
			return Err(ParseGitRevisionError::InvalidLength { got: value.len() });
		}

		Ok(Self {
			bytes: array::try_from_fn(|idx| {
				let substr = value
					.get(idx * 2..(idx + 1) * 2)
					.unwrap_or_else(|| panic!("we checked the input's length"));

				u8::from_str_radix(substr, 16).map_err(ParseGitRevisionError::ParseHexDigit)
			})?,
		})
	}
}

impl_rand!(GitRevision => |rng| GitRevision { bytes: rng.random() });

impl serde::Serialize for GitRevision
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		format_args!("{self}").serialize(serializer)
	}
}

impl<'de> serde::Deserialize<'de> for GitRevision
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		struct GitRevisionVisitor;

		impl de::Visitor<'_> for GitRevisionVisitor
		{
			type Value = GitRevision;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
			{
				fmt.write_str("a git revision")
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
					.map(|bytes| GitRevision { bytes })
					.map_err(|_| E::invalid_length(value.len(), &self))
			}
		}

		if deserializer.is_human_readable() {
			deserializer.deserialize_str(GitRevisionVisitor)
		} else {
			deserializer.deserialize_bytes(GitRevisionVisitor)
		}
	}
}

impl_sqlx!(GitRevision => {
	Type as [u8];
	Encode<'q, 'a> as &'a [u8] = |checksum| checksum.as_bytes();
	Decode<'r> as &'r [u8] = |bytes| {
		<&[u8] as TryInto<[u8; RAW_LEN]>>::try_into(bytes)
			.map(|bytes| GitRevision { bytes })
	};
});

impl_utoipa!(GitRevision => {
	Object::builder()
		.description(Some("a git revision"))
		.schema_type(schema::Type::String)
		.min_length(Some(STR_LEN))
		.max_length(Some(STR_LEN))
		.examples(["24bfd2242fc46340c95574468d78af687dea0e93"])
});
