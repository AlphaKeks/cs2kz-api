use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

#[derive(
	Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type, ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct WorkshopId(u32);

impl_rand!(WorkshopId => |rng| WorkshopId(rng.random::<u32>()));
