use {
	serde::{Deserialize, Serialize},
	std::net::Ipv4Addr,
};

#[derive(Debug, Display, Clone, Copy, From, Into, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PlayerIp(Ipv4Addr);
