use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

#[derive(
	Debug,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	From,
	Into,
	Serialize,
	Deserialize,
	ToSchema,
	sqlx::Type,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct Teleports(u32);

impl Teleports
{
	pub fn as_u32(self) -> u32
	{
		self.0
	}
}
