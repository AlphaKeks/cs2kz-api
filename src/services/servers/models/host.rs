//! CS2 server hosts.
//!
//! CS2 servers are allowed to use both IPv4/IPv6 and full domain names. This
//! module defines a `Host` type that encapsulates any of these 3, and can
//! encode/decode them properly.

use std::convert::Infallible;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A CS2 server host.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Host
{
	/// An IPv4 address.
	Ipv4(Ipv4Addr),

	/// An IPv6 address.
	Ipv6(Ipv6Addr),

	/// A domain.
	Domain(String),
}

impl FromStr for Host
{
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		if let Ok(ip) = value.parse::<Ipv4Addr>() {
			return Ok(Self::Ipv4(ip));
		}

		if let Ok(ip) = value.parse::<Ipv6Addr>() {
			return Ok(Self::Ipv6(ip));
		}

		Ok(Self::Domain(value.to_owned()))
	}
}

impl From<Ipv4Addr> for Host
{
	fn from(value: Ipv4Addr) -> Self
	{
		Self::Ipv4(value)
	}
}

impl From<Ipv6Addr> for Host
{
	fn from(value: Ipv6Addr) -> Self
	{
		Self::Ipv6(value)
	}
}

impl From<String> for Host
{
	fn from(value: String) -> Self
	{
		if let Ok(ip) = value.parse::<Ipv4Addr>() {
			return Self::Ipv4(ip);
		}

		if let Ok(ip) = value.parse::<Ipv6Addr>() {
			return Self::Ipv6(ip);
		}

		Self::Domain(value)
	}
}

impl<DB> sqlx::Type<DB> for Host
where
	DB: sqlx::Database,
	String: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		<String as sqlx::Type<DB>>::type_info()
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for Host
where
	DB: sqlx::Database,
	String: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::database::HasArguments<'q>>::ArgumentBuffer,
	) -> sqlx::encode::IsNull
	{
		match self {
			Host::Ipv4(ip) => <String as sqlx::Encode<DB>>::encode_by_ref(&ip.to_string(), buf),
			Host::Ipv6(ip) => <String as sqlx::Encode<DB>>::encode_by_ref(&ip.to_string(), buf),
			Host::Domain(domain) => <String as sqlx::Encode<DB>>::encode_by_ref(domain, buf),
		}
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for Host
where
	DB: sqlx::Database,
	&'r str: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::database::HasValueRef<'r>>::ValueRef,
	) -> Result<Self, sqlx::error::BoxDynError>
	{
		<&'r str as sqlx::Decode<'r, DB>>::decode(value)?
			.parse()
			.map_err(Into::into)
	}
}
