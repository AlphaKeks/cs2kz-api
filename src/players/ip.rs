use std::net::Ipv4Addr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Display, Clone, Copy, From, Into, Serialize, Deserialize, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct PlayerIp(Ipv4Addr);
