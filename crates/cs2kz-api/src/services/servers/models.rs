use std::convert::Infallible;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;

use cs2kz::SteamID;
use serde::{Deserialize, Serialize, Serializer};
use uuid::Uuid;

make_id! {
	/// An ID uniquely identifying a KZ server.
	pub struct ServerID(u16);
}

/// An access key for CS2 servers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(transparent)]
pub struct AccessKey(Uuid);

impl AccessKey
{
	/// Generates a new [`AccessKey`].
	pub fn new() -> Self
	{
		Self(Uuid::new_v4())
	}

	/// Checks if this is a valid key.
	pub fn is_valid(&self) -> bool
	{
		!self.0.is_nil()
	}
}

#[derive(Debug, Error)]
pub enum ParseAccessKeyError
{
	#[error("failed to parse access key: {0}")]
	Parse(#[from] uuid::Error),

	#[error("all zeros is not a valid access key")]
	IsNil,
}

impl FromStr for AccessKey
{
	type Err = ParseAccessKeyError;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		let raw = value.parse::<Uuid>()?;

		if raw.is_nil() {
			return Err(ParseAccessKeyError::IsNil);
		}

		Ok(Self(raw))
	}
}

impl Serialize for AccessKey
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.0.as_hyphenated().serialize(serializer)
	}
}

sql_type!(AccessKey as Uuid => {
	encode_by_ref: |self| &self.0,
	encode: |self| self.0,
	decode: |value| Ok(Self(value)),
});

/// A CS2 server host.
///
/// Servers are allowed to use arbitrary domains in addition to just raw IP addresses.
/// This type takes care of properly encoding/decoding these hosts.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

impl fmt::Display for Host
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
	{
		match self {
			Self::Ipv4(ipv4) => fmt::Display::fmt(ipv4, f),
			Self::Ipv6(ipv6) => fmt::Display::fmt(ipv6, f),
			Self::Domain(domain) => fmt::Display::fmt(domain, f),
		}
	}
}

impl FromStr for Host
{
	type Err = Infallible;

	fn from_str(value: &str) -> Result<Self, Self::Err>
	{
		match value.parse::<IpAddr>() {
			Ok(IpAddr::V4(ipv4)) => Ok(Self::Ipv4(ipv4)),
			Ok(IpAddr::V6(ipv6)) => Ok(Self::Ipv6(ipv6)),
			Err(_) => Ok(Self::Domain(value.to_owned())),
		}
	}
}

sql_type!(Host as String => {
	encode_by_ref: |self| &self.to_string(),
	encode: |self| self.to_string(),
	decode: |value| Ok(value.parse().unwrap_or_else(|err| match err {})),
});

/// An identifier for a CS2 server.
#[derive(Debug, Clone)]
pub enum ServerIdentifier
{
	/// A server ID.
	ID(ServerID),

	/// A server name.
	Name(String),
}

impl<'de> Deserialize<'de> for ServerIdentifier
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		use serde::de;

		#[derive(Debug, Deserialize)]
		#[serde(untagged)]
		enum Helper
		{
			Int(u16),
			Str(String),
		}

		Helper::deserialize(deserializer).and_then(|v| match v {
			Helper::Int(int) => TryFrom::try_from(int)
				.map(ServerID)
				.map(Self::ID)
				.map_err(de::Error::custom),

			Helper::Str(str) => {
				if let Ok(id) = str.parse::<ServerID>() {
					Ok(Self::ID(id))
				} else {
					Ok(Self::Name(str))
				}
			}
		})
	}
}

/// Information about a CS2 server owner.
#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct ServerOwner
{
	/// The user's SteamID.
	#[sqlx(rename = "server_owner_id")]
	pub steam_id: SteamID,

	/// The user's name.
	#[sqlx(rename = "server_owner_name")]
	pub name: String,
}
