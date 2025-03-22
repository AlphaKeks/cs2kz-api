use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
	Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct WorkshopId(u32);
