use std::net::{Ipv4Addr, Ipv6Addr};

use crate::database;

#[derive(Debug, Display, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(untagged)]
pub enum ServerHost {
	Ipv4(Ipv4Addr),
	Ipv6(Ipv6Addr),
	Domain(String),
}

database::macros::wrap!(ServerHost as str => {
	get: |self| &*self.to_string();
	make: |value| Ok({
		value.parse()
			.map(Self::Ipv4)
			.or_else(|_| value.parse().map(Self::Ipv6))
			.unwrap_or_else(|_| Self::Domain(value.to_owned()))
	});
});
