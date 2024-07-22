//! Extensions for IP address types from [`std::net`].

use std::net::{IpAddr, Ipv6Addr};

/// Extensions for [`std::net::IpAddr`].
#[sealed]
pub trait AddrExt
{
	/// Turns this [`IpAddr`] into an [`Ipv6Addr`].
	///
	/// If `self` is an IPv4 address, it will be mapped to an IPv6 address.
	fn to_v6(&self) -> Ipv6Addr;

	/// Turns this [`Ipv6Addr`] into an [`IpAddr`].
	///
	/// If `self` is a mapped IPv4 address, it will be mapped back to a normal
	/// IPv4 address.
	fn from_v6(ipv6: Ipv6Addr) -> Self;
}

#[sealed]
impl AddrExt for IpAddr
{
	fn to_v6(&self) -> Ipv6Addr
	{
		match self {
			IpAddr::V4(ipv4) => ipv4.to_ipv6_mapped(),
			IpAddr::V6(ipv6) => *ipv6,
		}
	}

	fn from_v6(ipv6: Ipv6Addr) -> Self
	{
		if let Some(ipv4) = ipv6.to_ipv4_mapped() {
			Self::V4(ipv4)
		} else {
			Self::V6(ipv6)
		}
	}
}
