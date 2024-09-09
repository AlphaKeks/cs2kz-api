use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpAddr(std::net::Ipv6Addr);

impl IpAddr
{
	pub fn localhost_v4() -> Self
	{
		std::net::Ipv4Addr::LOCALHOST.into()
	}

	pub fn localhost_v6() -> Self
	{
		std::net::Ipv6Addr::LOCALHOST.into()
	}

	pub fn as_ipv4(&self) -> Option<std::net::Ipv4Addr>
	{
		self.0.to_ipv4_mapped()
	}

	pub fn as_ipv6(&self) -> std::net::Ipv6Addr
	{
		self.0
	}
}

impl From<std::net::Ipv4Addr> for IpAddr
{
	fn from(ip: std::net::Ipv4Addr) -> Self
	{
		Self(ip.to_ipv6_mapped())
	}
}

impl From<std::net::Ipv6Addr> for IpAddr
{
	fn from(ip: std::net::Ipv6Addr) -> Self
	{
		Self(ip)
	}
}

impl From<std::net::IpAddr> for IpAddr
{
	fn from(ip: std::net::IpAddr) -> Self
	{
		match ip {
			std::net::IpAddr::V4(ip) => ip.into(),
			std::net::IpAddr::V6(ip) => ip.into(),
		}
	}
}

impl From<IpAddr> for std::net::IpAddr
{
	fn from(IpAddr(ip): IpAddr) -> Self
	{
		match ip.to_ipv4_mapped() {
			None => Self::V6(ip),
			Some(ip) => Self::V4(ip),
		}
	}
}

impl fmt::Display for IpAddr
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match self.as_ipv4() {
			None => fmt::Display::fmt(&self.as_ipv6(), f),
			Some(ipv4) => fmt::Display::fmt(&ipv4, f),
		}
	}
}

impl Serialize for IpAddr
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		std::net::IpAddr::from(*self).serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for IpAddr
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		std::net::IpAddr::deserialize(deserializer).map(Self::from)
	}
}

sql_type!(IpAddr as std::net::Ipv6Addr => {
	encode_by_ref: |self| &self.as_ipv6(),
	encode: |self| self.as_ipv6(),
	decode: |value| Ok(value.into()),
});
