use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase")]
pub enum Mode
{
	Vanilla,
	Classic,
	KZTimer,
	SimpleKZ,

	#[serde(rename = "vanilla-csgo")]
	#[sqlx(rename = "vanilla-csgo")]
	VanillaCSGO,
}
