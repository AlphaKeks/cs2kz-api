use {
	serde::{Deserialize, Deserializer, Serialize},
	std::borrow::Cow,
	utoipa::{
		PartialSchema,
		ToSchema,
		openapi::{
			Object,
			RefOr,
			SchemaFormat,
			schema::{self, KnownFormat, Schema},
		},
	},
};

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct PaginationResponse<T>
{
	total: u64,

	#[debug("{}", values.len())]
	values: Vec<T>,
}

impl<T> PaginationResponse<T>
{
	pub(crate) fn new(total: u64) -> Self
	{
		Self { total, values: Vec::default() }
	}
}

impl<T> Extend<T> for PaginationResponse<T>
{
	fn extend<I>(&mut self, iter: I)
	where
		I: IntoIterator<Item = T>,
	{
		self.values.extend(iter)
	}

	fn extend_one(&mut self, item: T)
	{
		self.values.extend_one(item)
	}

	fn extend_reserve(&mut self, additional: usize)
	{
		self.values.extend_reserve(additional)
	}
}

#[derive(Debug, Default, Clone, Copy)]
#[debug("Offset({value})")]
pub(crate) struct Offset
{
	value: u64,
}

impl Offset
{
	pub(crate) fn value(self) -> u64
	{
		self.value
	}
}

impl<'de> Deserialize<'de> for Offset
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<u64>::deserialize(deserializer)
			.map(Option::unwrap_or_default)
			.map(|value| Self { value })
	}
}

impl ToSchema for Offset
{
	fn name() -> Cow<'static, str>
	{
		Cow::Borrowed("Offset")
	}
}

impl PartialSchema for Offset
{
	fn schema() -> RefOr<Schema>
	{
		Object::builder()
			.schema_type(schema::Type::Integer)
			.format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt64)))
			.into()
	}
}

#[derive(Debug, Clone, Copy)]
#[debug("Limit({value})")]
pub(crate) struct Limit<const DEFAULT: u64, const MAX: u64 = { u64::MAX }>
{
	value: u64,
}

impl<const DEFAULT: u64, const MAX: u64> Limit<DEFAULT, MAX>
{
	pub(crate) fn value(self) -> u64
	{
		self.value
	}
}

impl<const DEFAULT: u64, const MAX: u64> Default for Limit<DEFAULT, MAX>
{
	fn default() -> Self
	{
		const { assert!(DEFAULT <= MAX) };
		Self { value: DEFAULT }
	}
}

impl<'de, const DEFAULT: u64, const MAX: u64> Deserialize<'de> for Limit<DEFAULT, MAX>
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		Option::<u64>::deserialize(deserializer).map(|value| match value {
			None => Self { value: DEFAULT },
			Some(value) if value <= MAX => Self { value },
			Some(_) => Self { value: MAX },
		})
	}
}

impl<const DEFAULT: u64, const MAX: u64> ToSchema for Limit<DEFAULT, MAX>
{
	fn name() -> Cow<'static, str>
	{
		Cow::Owned(format!("Limit_{DEFAULT}_{MAX}"))
	}
}

impl<const DEFAULT: u64, const MAX: u64> PartialSchema for Limit<DEFAULT, MAX>
{
	fn schema() -> RefOr<Schema>
	{
		Object::builder()
			.schema_type(schema::Type::Integer)
			.format(Some(SchemaFormat::KnownFormat(KnownFormat::UInt64)))
			.maximum(Some(MAX))
			.default(Some(DEFAULT.into()))
			.into()
	}
}
