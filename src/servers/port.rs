use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema)]
#[serde(transparent)]
#[sqlx(transparent)]
#[schema(example = 27015)]
pub struct ServerPort(u16);

impl_rand!(ServerPort => |rng| ServerPort(rng.random::<u16>()));
