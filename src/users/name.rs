use {
	serde::Serialize,
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, ToSchema)]
#[serde(transparent)]
#[schema(value_type = str, example = "AlphaKeks")]
pub struct Username(Arc<str>);

#[non_exhaustive]
#[derive(Debug, Display, Error)]
#[display("invalid username: {_variant}")]
pub enum InvalidUsername
{
	#[display("may not be empty")]
	Empty,
}

impl Username
{
	fn validate(value: &str) -> Result<(), InvalidUsername>
	{
		if value.is_empty() {
			return Err(InvalidUsername::Empty);
		}

		Ok(())
	}

	pub fn as_str(&self) -> &str
	{
		&self.0
	}
}

impl FromStr for Username
{
	type Err = InvalidUsername;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		Self::validate(value).map(|()| Self(value.into()))
	}
}

impl_sqlx!(Username => {
	Type as str;
	Encode<'q, 'a> as &'a str = |username| username.as_str();
	Decode<'r> as String = |value| {
		Username::validate(&value)
			.map(|()| Username(value.into()))
	};
});
