use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[repr(i8)]
#[derive(
	Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, ToSchema, clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum MapState
{
	/// The map was never approved and has been abandoned.
	Graveyard,

	/// The map is currently work-in-progress.
	WIP,

	/// The map has been submitted for approval.
	Pending,

	/// The map has been approved.
	Approved,

	/// The map was never approved but is considered "completed" by the mapper.
	Completed,
}

impl MapState
{
	/// Returns whether a map in the given state is "frozen".
	///
	/// A "frozen" map's state may only be updated by the API itself or admins,
	/// not the mapper.
	pub fn is_frozen(&self) -> bool
	{
		matches!(self, Self::Pending | Self::Approved | Self::Completed)
	}
}
