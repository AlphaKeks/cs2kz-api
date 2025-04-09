use {
	serde::{Deserialize, Serialize},
	utoipa::ToSchema,
};

/// A user's server budget
///
/// This indicates how many servers they are still allowed to create.
#[derive(
	Debug,
	Display,
	Clone,
	Copy,
	PartialEq,
	Eq,
	PartialOrd,
	Ord,
	Hash,
	From,
	Into,
	Serialize,
	Deserialize,
	sqlx::Type,
	ToSchema,
)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct ServerBudget(u16);

impl ServerBudget
{
	pub fn is_exhausted(&self) -> bool
	{
		self.0 == 0
	}
}
