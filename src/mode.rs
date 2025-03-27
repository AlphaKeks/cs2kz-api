use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Mode
{
	Vanilla,
	Classic,
	KZTimer,
	SimpleKZ,

	#[serde(rename = "vanilla-csgo")]
	VanillaCSGO,
}
