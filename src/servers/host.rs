use std::{
	error::Error,
	fmt,
	net::{IpAddr, Ipv4Addr, Ipv6Addr},
	sync::Arc,
};

use addr::parse_domain_name;
use serde::{Deserialize, Deserializer, Serialize, de};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
#[serde(untagged)]
pub enum ServerHost
{
	#[schema(value_type = str, format = Ipv4)]
	Ipv4(Ipv4Addr),

	#[schema(value_type = str, format = Ipv6)]
	Ipv6(Ipv6Addr),

	#[schema(value_type = str, format = Hostname)]
	Domain(Arc<str>),
}

impl<'de> Deserialize<'de> for ServerHost
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct ServerHostVisitor;

		impl de::Visitor<'_> for ServerHostVisitor
		{
			type Value = ServerHost;

			fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result
			{
				fmt.write_str("a server IP or domain name")
			}

			fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				if let Ok(ip) = value.parse::<IpAddr>() {
					return Ok(match ip {
						IpAddr::V4(ipv4) => ServerHost::Ipv4(ipv4),
						IpAddr::V6(ipv6) => ServerHost::Ipv6(ipv6),
					});
				}

				parse_domain_name(value)
					.map(|name| ServerHost::Domain(name.as_str().into()))
					.map_err(E::custom)
			}
		}

		deserializer.deserialize_str(ServerHostVisitor)
	}
}

impl<DB> sqlx::Type<DB> for ServerHost
where
	DB: sqlx::Database,
	IpAddr: sqlx::Type<DB>,
	str: sqlx::Type<DB>,
{
	fn type_info() -> <DB as sqlx::Database>::TypeInfo
	{
		// TODO(AlphaKeks): double-check if this is sufficient
		<str as sqlx::Type<DB>>::type_info()
	}

	fn compatible(ty: &<DB as sqlx::Database>::TypeInfo) -> bool
	{
		<IpAddr as sqlx::Type<DB>>::compatible(ty) || <str as sqlx::Type<DB>>::compatible(ty)
	}
}

impl<'q, DB> sqlx::Encode<'q, DB> for ServerHost
where
	DB: sqlx::Database,
	Ipv4Addr: sqlx::Encode<'q, DB>,
	Ipv6Addr: sqlx::Encode<'q, DB>,
	for<'a> &'a str: sqlx::Encode<'q, DB>,
{
	fn encode_by_ref(
		&self,
		buf: &mut <DB as sqlx::Database>::ArgumentBuffer<'q>,
	) -> Result<sqlx::encode::IsNull, Box<dyn Error + Send + Sync>>
	{
		match *self {
			ServerHost::Ipv4(ref ipv4) => ipv4.encode_by_ref(buf),
			ServerHost::Ipv6(ref ipv6) => ipv6.encode_by_ref(buf),
			ServerHost::Domain(ref domain) => (&**domain).encode_by_ref(buf),
		}
	}

	fn produces(&self) -> Option<<DB as sqlx::Database>::TypeInfo>
	{
		match *self {
			ServerHost::Ipv4(ref ipv4) => ipv4.produces(),
			ServerHost::Ipv6(ref ipv6) => ipv6.produces(),
			ServerHost::Domain(ref domain) => (&**domain).produces(),
		}
	}

	fn size_hint(&self) -> usize
	{
		match *self {
			ServerHost::Ipv4(ref ipv4) => ipv4.size_hint(),
			ServerHost::Ipv6(ref ipv6) => ipv6.size_hint(),
			ServerHost::Domain(ref domain) => (&**domain).size_hint(),
		}
	}
}

impl<'r, DB> sqlx::Decode<'r, DB> for ServerHost
where
	DB: sqlx::Database,
	<DB as sqlx::Database>::ValueRef<'r>: Clone,
	IpAddr: sqlx::Decode<'r, DB>,
	String: sqlx::Decode<'r, DB>,
{
	fn decode(
		value: <DB as sqlx::Database>::ValueRef<'r>,
	) -> Result<Self, Box<dyn Error + Send + Sync>>
	{
		if let Ok(ip) = IpAddr::decode(value.clone()) {
			return Ok(match ip {
				IpAddr::V4(ipv4) => Self::Ipv4(ipv4),
				IpAddr::V6(ipv6) => Self::Ipv6(ipv6),
			});
		}

		let decoded = String::decode(value)?;

		if let Err(err) = parse_domain_name(&decoded) {
			return Err(err.to_string().into());
		}

		Ok(Self::Domain(decoded.into()))
	}
}
