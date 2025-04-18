use {
	serde::{Deserialize, Serialize},
	std::{str::FromStr, sync::Arc},
	utoipa::ToSchema,
};

#[derive(Debug, Display, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[schema(value_type = str, format = Email)]
pub struct EmailAddress(Arc<lettre::Address>);

#[derive(Debug, Display, Error, From)]
#[display("failed to parse email address: {_0}")]
pub struct ParseEmailAddressError(lettre::address::AddressError);

impl EmailAddress
{
	pub fn as_str(&self) -> &str
	{
		lettre::Address::as_ref(&*self.0)
	}
}

impl AsRef<lettre::Address> for EmailAddress
{
	fn as_ref(&self) -> &lettre::Address
	{
		&*self.0
	}
}

impl FromStr for EmailAddress
{
	type Err = ParseEmailAddressError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		value
			.parse::<lettre::Address>()
			.map(Arc::new)
			.map(Self)
			.map_err(ParseEmailAddressError)
	}
}

impl_sqlx!(EmailAddress => {
	Type as str;
	Encode<'q, 'a> as &'a str = |email| email.as_str();
	Decode<'r> as &'r str = |value| value.parse();
});
